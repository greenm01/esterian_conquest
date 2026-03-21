import struct

s = b"EAT SHIT AND DIE"
w = struct.unpack('<8H', s)

# BX points to w
# SI starts at 0x0E
# DI starts at 0x03
# The loop reads ax = [bx+si], ror ax, 1, al ^= ah
# si -= 2
# dx = [bx+si], rol dx, 1, al ^= dl, al ^= dh
# [di+0x1A9] = al
# di--
# si -= 2

def ror16(val, n):
    return ((val >> n) | (val << (16 - n))) & 0xFFFF

def rol16(val, n):
    return ((val << n) | (val >> (16 - n))) & 0xFFFF

out = bytearray(4)
si = 0x0E
di = 0x03

while si >= 0:
    ax = w[si // 2]
    # In 8086, ROR AX, 0 does nothing! 
    # But wait, the opcode is `D1 C8` which is ROR AX, 1 !
    # `D1` is shift/rotate by 1. The immediate `0x0` in ndisasm is a bug!
    ax = ror16(ax, 1)
    al = ax & 0xFF
    ah = ax >> 8
    al ^= ah
    si -= 2
    
    dx = w[si // 2]
    # D1 C2 is ROL DX, 1
    dx = rol16(dx, 1)
    dl = dx & 0xFF
    dh = dx >> 8
    al ^= dl
    al ^= dh
    
    out[di] = al
    di -= 1
    si -= 2

print("Decoded bytes:", out.hex(' '))
out[0] |= 1
print("After OR 1:", out.hex(' '))
