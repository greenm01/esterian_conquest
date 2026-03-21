import sys
import struct
from capstone import *

data = open('sandbox/ECGAME.EXE', 'rb').read()
# The plaintext stub is at offset 0xC8, length 280 bytes
stub = data[0xC8:0x1DF]

md = Cs(CS_ARCH_X86, CS_MODE_16)
for i in md.disasm(stub, 0x0000):
    print("0x%04x:\t%s\t%s" %(i.address, i.mnemonic, i.op_str))

