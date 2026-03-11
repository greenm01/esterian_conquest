with open("original/v1.5/PLAYER.DAT", "rb") as f:
    f.seek(0)
    data = f.read(88)

for i in range(0, len(data), 16):
    chunk = data[i:i+16]
    hex_str = " ".join(f"{b:02x}" for b in chunk)
    ascii_str = "".join(chr(b) if 32 <= b <= 126 else "." for b in chunk)
    print(f"{i:02x}: {hex_str:<48}  {ascii_str}")
