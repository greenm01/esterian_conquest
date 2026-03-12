with open("/tmp/ecmaint-debug/MEMDUMP.BIN", "rb") as f:
    data = f.read()

for i in range(0, len(data) - 1):
    if data[i] == 0x2C and data[i+1] == 0x05:
        print(f"Found pointer 052C at 0x{i:X}")
