#!/bin/bash
# Wrapper script to launch Esterian Conquest (DOS) from Enigma BBS
# Usage: run_ec_dos.sh <dropfile_path> <node_number>

DROPFILE=$1
NODE=$2
GAME_DIR="/home/mag/dev/esterian_conquest/original/v1.5"

echo "$(date) - Launching door with $@" >> /tmp/ec-door.log

if [ -d "$DROPFILE" ]; then
    DROPFILE="$DROPFILE/DOOR.SYS"
fi

if [ -f "$DROPFILE" ]; then
    cp "$DROPFILE" "$GAME_DIR/DOOR.SYS"
    chmod 666 "$GAME_DIR/DOOR.SYS"
else
    echo "ERROR: Dropfile not found at $DROPFILE" >> /tmp/ec-door.log
fi

export SDL_VIDEODRIVER=dummy

# Ah! It looks like ECGAME is incorrectly parsing `/N:1` as a directory path
# `/N:\1\` instead of the node argument. The safest way to play this is to
# wrap the entire string in quotes or just avoid passing /N since we only have node 1 right now.
# Let's also drop `/N` completely since it defaults to Node 1.

dosbox-x -conf /dev/null \
  -fastlaunch \
  -nogui \
  -set "dosv=off" \
  -set "machine=vgaonly" \
  -set "core=normal" \
  -set "cputype=386_prefetch" \
  -set "cycles=fixed 3000" \
  -c "mount c $GAME_DIR" \
  -c "c:" \
  -c "ECGAME.EXE" \
  -c "exit" >> /tmp/ec-door.log 2>&1
