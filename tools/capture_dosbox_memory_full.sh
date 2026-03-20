#!/bin/bash
# Captures ALL readable memory from DOSBox-X into separate files.
# Usage: sudo ./capture_dosbox_memory_full.sh
set -euo pipefail

PID=$(pgrep -nf dosbox-x || true)
if [ -z "$PID" ]; then echo "DOSBox-X not running." >&2; exit 1; fi
echo "DOSBox-X PID: $PID"

python3 -c "
import struct, os

pid = $PID
maps = open(f'/proc/{pid}/maps').read()
mem = open(f'/proc/{pid}/mem', 'rb')

os.makedirs('/tmp/dosbox_regions', exist_ok=True)

sigs = [b'Runtime error', b'PLANETS.DAT', b'Esterian', b'COMMAND']
ivt_sig = None  # will find

idx = 0
for line in maps.strip().split('\n'):
    parts = line.split()
    if 'r' not in parts[1]: continue
    s, e = [int(x,16) for x in parts[0].split('-')]
    sz = e - s
    if sz < 0x1000 or sz > 200*1024*1024: continue
    try:
        mem.seek(s)
        chunk = mem.read(sz)
    except: continue

    # Check for interesting content
    hits = []
    for sig in sigs:
        p = chunk.find(sig)
        if p >= 0:
            hits.append(f'{sig.decode()}@{p:#x}')

    # Check for IVT (F000 segment in interrupt vectors)
    f000_count = 0
    if sz >= 0x100:
        for i in range(0, min(0x100, sz-2), 4):
            cs = struct.unpack_from('<H', chunk, i+2)[0]
            if cs == 0xF000:
                f000_count += 1

    # Check BIOS data area
    memsz = 0
    if sz > 0x414:
        memsz = struct.unpack_from('<H', chunk, 0x413)[0]

    if hits or f000_count >= 3 or memsz == 640:
        label = f'region_{idx:02d}_{s:012x}_{sz:08x}'
        path = f'/tmp/dosbox_regions/{label}.bin'
        open(path, 'wb').write(chunk)
        print(f'{parts[0]} ({sz/1024:.0f}KB): {\" \".join(hits)} f000={f000_count} memsz={memsz}')
        idx += 1

print(f'Saved {idx} interesting regions to /tmp/dosbox_regions/')
"
