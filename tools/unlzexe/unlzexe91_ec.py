#!/usr/bin/env python3
"""LZEXE 0.91 decompressor for encrypted-stub Esterian Conquest EXEs.

Scans the binary for the start of the compressed bitstream (bypassing the
encrypted stub parameters) and outputs the original unencrypted MZ executable.
"""
import struct
import sys
from pathlib import Path

def attempt_decompress(data, start_pos):
    pos = start_pos
    def getword():
        nonlocal pos
        if pos + 2 > len(data): raise ValueError("EOF")
        w = data[pos] | (data[pos + 1] << 8)
        pos += 2
        return w

    def getbyte():
        nonlocal pos
        if pos >= len(data): raise ValueError("EOF")
        b = data[pos]
        pos += 1
        return b

    bitbuf = getword()
    bitcount = 16

    def getbit():
        nonlocal bitbuf, bitcount
        b = bitbuf & 1
        bitcount -= 1
        if bitcount == 0:
            bitbuf = getword()
            bitcount = 16
        else:
            bitbuf >>= 1
        return b

    window = bytearray(0x10000)
    wp = 0
    output = bytearray()
    iterations = 0

    while True:
        if getbit():
            b = getbyte()
            window[wp & 0xFFFF] = b
            output.append(b)
            wp += 1
        else:
            if not getbit():
                length = getbit() << 1
                length |= getbit()
                length += 2
                span = getbyte() | 0xFF00
            else:
                span_lo = getbyte()
                len_byte = getbyte()
                span = span_lo | ((len_byte & 0xF8) << 5) | 0xE000
                length = (len_byte & 0x07) + 2

                if length == 2:
                    length = getbyte()
                    if length == 0:
                        break  # end marker
                    if length == 1:
                        continue  # segment change marker
                    length += 1

            for _ in range(length):
                src = (wp + (span - 0x10000)) & 0xFFFF
                b = window[src]
                window[wp & 0xFFFF] = b
                output.append(b)
                wp += 1
                
        iterations += 1
        if iterations > 300000:
            raise ValueError("Too many iterations")

    return output

def decompress_lzexe91_ec(exe_path, out_path):
    data = Path(exe_path).read_bytes()
    mz = struct.unpack('<16H', data[:32])

    hdr_paras = mz[4]
    hdr_size = hdr_paras << 4
    cs_rel = mz[0x0B]
    stub_file_off = hdr_size + (cs_rel << 4)

    s = stub_file_off + 0x1B5
    assert data[s] == 0x4D and data[s+1] == 0x5A, f"No MZ at +0x1B5 (got {data[s]:02x}{data[s+1]:02x})"
    orig = struct.unpack('<16H', data[s:s+32])

    o_cblp, o_cp, o_crlc, o_cparhdr, o_minalloc, o_maxalloc, o_ss, o_sp = orig[1:9]
    o_ip, o_cs = orig[0x0A], orig[0x0B]

    print(f"Original: CS:IP={o_cs:04x}:{o_ip:04x} SS:SP={o_ss:04x}:{o_sp:04x} relocs={o_crlc}")

    output = None
    for offset in range(hdr_size, stub_file_off, 16):
        try:
            out = attempt_decompress(data, offset)
            if len(out) > 10000:
                print(f"Success! Decompressed {len(out)} bytes from offset 0x{offset:x}")
                output = out
                break
        except Exception:
            pass

    if not output:
        print("Failed to find valid LZEXE data stream.")
        return False

    # Adjust original header fields for standard MZ (no LZEXE anymore)
    header = struct.pack('<16H',
        0x5A4D, o_cblp, o_cp, o_crlc, o_cparhdr, o_minalloc, o_maxalloc,
        o_ss, o_sp, 0, o_ip, o_cs, 0x001C, 0, 0, 0
    )

    result = header + bytes(output)
    
    if o_cblp:
        expected = (o_cp - 1) * 512 + o_cblp
    else:
        expected = o_cp * 512
        
    actual = len(result)
    if actual != expected:
        print(f"Padding from {actual} to {expected} bytes")
        if actual < expected:
            result += b'\x00' * (expected - actual)
        else:
            result = result[:expected]

    Path(out_path).write_bytes(result)
    print(f"Written {out_path} ({len(result)} bytes)")

    for sig in [b'Runtime error', b'Esterian', b'PLANETS.DAT', b'PLAYER.DAT', b'ECGAME', b'ECMAINT']:
        if result.find(sig) >= 0:
            print(f"  Found '{sig.decode()}'")

    return True

if __name__ == '__main__':
    if len(sys.argv) < 3:
        print(f"usage: {sys.argv[0]} input.exe output.exe")
        sys.exit(1)
    decompress_lzexe91_ec(sys.argv[1], sys.argv[2])
