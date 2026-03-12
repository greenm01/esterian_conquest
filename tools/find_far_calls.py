import sys

with open("/tmp/ecmaint-debug/MEMDUMP.BIN", "rb") as f:
    data = f.read()

for i in range(len(data) - 4):
    if data[i] == 0x9A and data[i+1] == 0x7C and data[i+2] == 0x99:
        print(f"Found FAR call at offset {hex(i)}")
        # print segment
        seg = int.from_bytes(data[i+3:i+5], byteorder='little')
        print(f"Segment: {hex(seg)}")

