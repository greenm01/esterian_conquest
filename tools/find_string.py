with open('ecmaint.bin', 'rb') as f:
    data = f.read()

idx = data.find(b'failed integrity check')
if idx != -1:
    print(f"Found string at {hex(idx)}")
    # Find xrefs (instructions like mov dx, offset or push offset)
    # The string is at offset `idx`. In a DOS COM/EXE, data segment base is unknown, but usually offset is relative to DS.
    # We can just search for the two bytes `idx & 0xFFFF` (little endian) in the binary.
    offset = idx & 0xFFFF
    target = bytes([offset & 0xFF, (offset >> 8) & 0xFF])
    print(f"Searching for pointer: {target.hex()}")
    for i in range(len(data) - 1):
        if data[i] == target[0] and data[i+1] == target[1]:
            print(f"Possible pointer at {hex(i)}")
else:
    print("String not found.")
