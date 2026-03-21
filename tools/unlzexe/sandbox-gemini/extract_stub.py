import sys
data = open('sandbox/ECGAME.EXE', 'rb').read()
open('sandbox/stub.bin', 'wb').write(data[0xC8:0x1DF])