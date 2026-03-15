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

# Clean up any previous drop files so ECGAME isn't confused
rm -f "$GAME_DIR/CHAIN.TXT"
rm -f "$GAME_DIR/DOOR.SYS"

if [ -f "$DROPFILE" ]; then
    cp "$DROPFILE" "$GAME_DIR/DOOR.SYS"
    chmod 666 "$GAME_DIR/DOOR.SYS"
else
    echo "ERROR: Dropfile not found at $DROPFILE" >> /tmp/ec-door.log
fi

export SDL_VIDEODRIVER=dummy

# If you don't specify /D:, ECGAME prioritizes CHAIN.TXT in the current dir.
# If we want it to parse DOOR.SYS, we MUST give it the directory!
# The syntax ECGAME wants is exactly: /D:C:\
# But in DOSBox `-c "ECGAME.EXE /D:C:\ "` might get its backslash eaten by bash.
# We will use two backslashes in the mount command to ensure it arrives at DOS.

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
  -c "ECGAME.EXE /D:C:\\" \
  -c "exit" >> /tmp/ec-door.log 2>&1
