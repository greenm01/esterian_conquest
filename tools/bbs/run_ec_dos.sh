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

rm -f "$GAME_DIR/CHAIN.TXT"
rm -f "$GAME_DIR/DOOR.SYS"
rm -f "$GAME_DIR/DORINFO1.DEF"

if [ -f "$DROPFILE" ]; then
    cp "$DROPFILE" "$GAME_DIR/DOOR.SYS"
    chmod 666 "$GAME_DIR/DOOR.SYS"
else
    echo "ERROR: Dropfile not found at $DROPFILE" >> /tmp/ec-door.log
fi

export SDL_VIDEODRIVER=dummy

# Enigma generates DOOR.SYS by default based on our dropFileType config.
# Let's switch the strategy. The absolute most reliable way to launch ECGAME
# in DOSBox-X without bash escaping issues on backslashes is to write a tiny
# .BAT file on the fly and launch THAT inside dosbox.

cat << 'BAT' > "$GAME_DIR/RUN.BAT"
@ECHO OFF
C:
ECGAME.EXE /D:C:\
BAT

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
  -c "RUN.BAT" \
  -c "exit" >> /tmp/ec-door.log 2>&1
