with open("/tmp/ecmaint-debug/MEMDUMP.BIN", "rb") as f:
    data = f.read()

target = 0x2997C
for i in range(0, len(data) - 4):
    # check CALL NEAR (E8)
    if data[i] == 0xE8:
        offset = int.from_bytes(data[i+1:i+3], "little", signed=True)
        dest = i + 3 + offset
        if dest == target:
            print(f"Found CALL NEAR at 0x{i:X}")
            
    # check CALL FAR (9A)
    if data[i] == 0x9A:
        offset = int.from_bytes(data[i+1:i+3], "little", signed=False)
        segment = int.from_bytes(data[i+3:i+5], "little", signed=False)
        dest = segment * 16 + offset
        # Note: in MEMDUMP.BIN the segment is physical!
        if dest == target:
            print(f"Found CALL FAR at 0x{i:X} (seg {segment:X})")
