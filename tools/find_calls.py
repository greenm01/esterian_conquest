with open("/tmp/ecmaint-debug/MEMDUMP.BIN", "rb") as f:
    data = f.read()

target = 0x2997C
for i in range(0, len(data) - 3):
    if data[i] == 0xE8:
        # e8 offset
        offset = int.from_bytes(data[i+1:i+3], "little", signed=True)
        dest = i + 3 + offset
        if dest == target:
            print(f"Found CALL NEAR at 0x{i:X}")
