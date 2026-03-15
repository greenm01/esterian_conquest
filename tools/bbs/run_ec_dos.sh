#!/bin/bash
# Wrapper script to launch Esterian Conquest (DOS) from Enigma BBS
# Usage: run_ec_dos.sh <dropfile_path> <node_number>

DROPFILE=$1
NODE=$2
GAME_DIR="/home/mag/dev/esterian_conquest/original/v1.5"

echo "$(date) - Launching door with $@" >> /tmp/ec-door.log

# Enigma generates the dropfile in the path provided.
# If the path is a directory, append DOOR.SYS
if [ -d "$DROPFILE" ]; then
    DROPFILE="$DROPFILE/DOOR.SYS"
fi

echo "Using DROPFILE: $DROPFILE" >> /tmp/ec-door.log

if [ -f "$DROPFILE" ]; then
    cp "$DROPFILE" "$GAME_DIR/DOOR.SYS"
    chmod 666 "$GAME_DIR/DOOR.SYS"
else
    echo "ERROR: Dropfile not found at $DROPFILE" >> /tmp/ec-door.log
fi

export SDL_VIDEODRIVER=dummy

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
  -c "ECGAME.EXE /D:C:\\ /N:$NODE" \
  -c "exit" >> /tmp/ec-door.log 2>&1
