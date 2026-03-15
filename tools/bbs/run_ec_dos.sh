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

# The game's command line parsing is extremely specific about the path separator
# C:\ is what it expects, but escaping in bash -> dosbox can get mangled.
# Let's just use the default dropfile location by omitting the /D flag if DOOR.SYS is in the game dir.

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
  -c "ECGAME.EXE /N:$NODE" \
  -c "exit" >> /tmp/ec-door.log 2>&1
