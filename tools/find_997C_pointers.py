import sys

with open("/tmp/ecmaint-debug/MEMDUMP.BIN", "rb") as f:
    data = f.read()

for i in range(len(data) - 1):
    if data[i] == 0x7C and data[i+1] == 0x99:
        print(f"Found pointer 7C 99 at {hex(i)}")

