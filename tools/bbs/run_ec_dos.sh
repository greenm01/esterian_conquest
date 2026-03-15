#!/bin/bash
# Wrapper script to launch Esterian Conquest (DOS) from Enigma BBS
# Usage: run_ec_dos.sh <dropfile_path> <node_number> <port>

DROPFILE=$1
NODE=$2
PORT=$3
GAME_DIR="/home/mag/dev/esterian_conquest/original/v1.5"

echo "$(date) - Launching door with $@" >> /tmp/ec-door.log

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

# The ECGAME docs state: "When running ECGAME, you can specify the door file name"
# Ex: ECGAME \BBS\DOOR.SYS
# So we pass "C:\DOOR.SYS" directly, without /D or /N flags!

cat << 'BAT' > "$GAME_DIR/RUN.BAT"
@ECHO OFF
C:
ECGAME.EXE C:\DOOR.SYS
BAT

dosbox-x -conf /dev/null \
  -fastlaunch \
  -nogui \
  -set "dosv=off" \
  -set "machine=vgaonly" \
  -set "core=normal" \
  -set "cputype=386_prefetch" \
  -set "cycles=fixed 3000" \
  -c "serial1=nullmodem server:127.0.0.1 port:$PORT" \
  -c "mount c $GAME_DIR" \
  -c "c:" \
  -c "RUN.BAT" \
  -c "exit" >> /tmp/ec-door.log 2>&1
