import sys

with open('/tmp/ecmaint-debug-token/MEMDUMP.BIN', 'rb') as f:
    mem = f.read()

# Look for FAR return address to 3159:XXXX
for i in range(0, len(mem) - 4, 2):
    if mem[i+2:i+4] == b'\x59\x31':
        ip = int.from_bytes(mem[i:i+2], 'little')
        # We know IP should be somewhere. Linear address = 3159 * 16 + ip = 0x31590 + ip
        # Let's print all of them.
        print(f"Found match at linear {hex(i)}: Return to 3159:{hex(ip)}")
