import sys

with open("/tmp/ecmaint-debug/MEMDUMP.BIN", "rb") as f:
    data = f.read()

target_offset = 0x997C
cs_base = 0x29450

for i in range(cs_base, min(cs_base + 0x10000, len(data))):
    if data[i] == 0xE9:
        rel = int.from_bytes(data[i+1:i+3], byteorder='little', signed=True)
        target = (i - cs_base + 3 + rel) & 0xFFFF
        if target == target_offset:
            print(f"Found relative JMP at offset {hex(i - cs_base)} (file offset {hex(i)})")

