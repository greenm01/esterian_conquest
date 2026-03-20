#!/bin/bash
# Captures the decompressed ECGAME.EXE memory from a running DOSBox-X instance.
# Usage: sudo ./capture_dosbox_memory.sh
# DOSBox-X must already be running with ECGAME loaded.
set -euo pipefail

PID=$(pgrep -nf dosbox-x || true)
if [ -z "$PID" ]; then
    echo "DOSBox-X not running. Start it with ECGAME first." >&2
    exit 1
fi

echo "DOSBox-X PID: $PID"

python3 -c "
import struct

pid = $PID
maps = open(f'/proc/{pid}/maps').read()
mem = open(f'/proc/{pid}/mem', 'rb')

for line in maps.strip().split('\n'):
    parts = line.split()
    if 'r' not in parts[1]: continue
    s, e = [int(x,16) for x in parts[0].split('-')]
    sz = e - s
    if sz < 0x100000 or sz > 100*1024*1024: continue
    try:
        mem.seek(s)
        chunk = mem.read(sz)
    except: continue
    if chunk.find(b'Runtime error') >= 0 and sz > 0x100000:
        emu = chunk[:0x100000]
        open('/tmp/ecgame_1mb.bin', 'wb').write(emu)
        print(f'Region: {parts[0]} ({sz/1024/1024:.1f}MB)')
        for sig in [b'Runtime error', b'Turbo Pascal', b'PLANETS.DAT',
                    b'DATABASE.DAT', b'CHAIN.TXT', b'Esterian', b'ECGAME',
                    b'Borland', b'COMMAND', b'Conquest', b'SETUP.DAT',
                    b'PLAYER.DAT', b'FLEETS.DAT', b'BASES.DAT']:
            p = emu.find(sig)
            if p >= 0:
                print(f'  {sig.decode()}: 0x{p:05x} (seg ~{p>>4:04x})')
        print('Saved /tmp/ecgame_1mb.bin')
        break
"
