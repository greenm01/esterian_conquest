import sys

with open("/tmp/ecmaint-debug/MEMDUMP.BIN", "rb") as f:
    data = f.read()

target_offset = 0x997C
base = 0x20000

for i in range(base, base + 0x10000):
    if data[i] == 0xE8:
        # read next 2 bytes as signed short
        rel = int.from_bytes(data[i+1:i+3], byteorder='little', signed=True)
        target = (i - base + 3 + rel) & 0xFFFF
        if target == target_offset:
            print(f"Found relative call at offset {hex(i - base)}")

