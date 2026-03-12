import os

with open("/tmp/ecmaint-debug/MEMDUMP.BIN", "rb") as f:
    data = f.read()

# search for "Database.Tok"
idx = data.find(b"Database.Tok")
if idx != -1:
    print(f"Found Database.Tok at offset {hex(idx)}")
    # dump 100 bytes around it
    start = max(0, idx - 100)
    end = min(len(data), idx + 100)
    
    print("Strings block:")
    chunk = data[start:end]
    for i in range(0, len(chunk), 16):
        line = chunk[i:i+16]
        hex_str = " ".join(f"{b:02x}" for b in line)
        ascii_str = "".join(chr(b) if 32 <= b < 127 else "." for b in line)
        print(f"{hex(start + i)}: {hex_str:<48} {ascii_str}")
