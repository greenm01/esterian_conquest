import struct

def ror16(val, n):
    return ((val >> n) | (val << (16 - n))) & 0xFFFF

def rol16(val, n):
    return ((val << n) | (val >> (16 - n))) & 0xFFFF

def decode_params():
    s = b"EAT SHIT AND DIE"
    w = struct.unpack('<8H', s)
    
    out = bytearray(4)
    si = 0x0E
    di = 0x03
    
    while si >= 0:
        ax = w[si // 2]
        ax = ror16(ax, 1)
        al = ax & 0xFF
        ah = ax >> 8
        al ^= ah
        
        si -= 2
        if si < 0:
            break
            
        dx = w[si // 2]
        dx = rol16(dx, 1)
        dl = dx & 0xFF
        dh = dx >> 8
        al ^= dl
        al ^= dh
        
        out[di] = al
        di -= 1
        si -= 2
        
    out[0] |= 1
    return struct.unpack('<HH', out)

lenlz, decalage = decode_params()
print(f"Derived lenlz: 0x{lenlz:04x} ({lenlz})")
print(f"Derived decalage: 0x{decalage:04x} ({decalage})")

