import sys
import struct
from pathlib import Path

def try_decompress(data, start_pos):
    pos = start_pos
    def getword():
        nonlocal pos
        if pos + 2 > len(data): return 0
        w = data[pos] | (data[pos + 1] << 8)
        pos += 2
        return w

    def getbyte():
        nonlocal pos
        if pos >= len(data): return 0
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
        if pos >= len(data):
            return -1 # EOF
            
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
        if iterations > 200000: # Safety
            return -1

    return len(output)

def main():
    exe_path = sys.argv[1]
    data = Path(exe_path).read_bytes()
    hdr_size = (data[8] | (data[9] << 8)) << 4
    
    print(f"Header size: 0x{hdr_size:x}")
    print("Brute-forcing start offset...")
    
    # The compressed data stream is somewhere between the header and the stub
    # Try every 16-byte aligned offset
    for offset in range(hdr_size, len(data), 16):
        try:
            out_len = try_decompress(data, offset)
            if out_len > 20000: # We expect around 115k, but anything > 20k is a solid hit
                print(f"SUCCESS! Offset 0x{offset:x} decompressed to {out_len} bytes.")
                return
        except Exception:
            pass

if __name__ == '__main__':
    main()
