import sys

def decrypt_stub(data):
    stub = bytearray(data)
    
    # Layer 1: XOR key 0xAD, bytes +0x0F through +0x151
    # Wait, the eb ff trick means the loop modifies itself.
    # The standard XOR loop is usually a simple XOR over a range.
    for i in range(0x0F, 0x152):
        stub[i] ^= 0xAD
        
    # Layer 2: XOR key 0x3F, bytes +0x53 through +0x150
    for i in range(0x53, 0x151):
        stub[i] ^= 0x3F
        
    # Layer 3: Rolling XOR with initial key 0x25 (ROR 1 each iteration), bytes +0x150 down to +0x6C
    # ROR 1 of an 8-bit byte:
    def ror1(val):
        return ((val >> 1) | ((val & 1) << 7)) & 0xFF
        
    key = 0x25
    for i in range(0x150, 0x6B, -1):
        stub[i] ^= key
        key = ror1(key)
        
    return stub

data = open('sandbox/ECGAME.EXE', 'rb').read()
stub_base = 0x1C560
stub_data = data[stub_base:stub_base+544]

decrypted = decrypt_stub(stub_data)
open('sandbox/decrypted_stub.bin', 'wb').write(decrypted)
print("Decrypted stub written to decrypted_stub.bin")
