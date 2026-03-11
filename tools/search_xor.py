target = b"Game file"
data = open('original/v1.5/ECMAINT.EXE', 'rb').read()
found = False
for key in range(256):
    xored = bytes([c ^ key for c in target])
    if xored in data:
        print(f"Found XOR key: {hex(key)}")
        found = True

if not found:
    print("Not found with single-byte XOR.")