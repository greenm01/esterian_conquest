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

data = Path('sandbox/ECGAME.EXE').read_bytes()
for offset in range(0x200, 0x1000, 16):
    try:
        out = attempt_decompress(data, offset)
        if len(out) > 10000:
            print(f"Success! offset 0x{offset:x} yielded {len(out)} bytes")
            Path('sandbox/brute_out.bin').write_bytes(out)
            break
    except Exception:
        pass
