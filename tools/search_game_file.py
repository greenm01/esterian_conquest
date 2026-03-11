with open('ecmaint.bin', 'rb') as f:
    data = f.read()

idx = data.find(b'Game file')
if idx != -1:
    print(f"Found 'Game file' at {hex(idx)}")
else:
    print("Not found.")
