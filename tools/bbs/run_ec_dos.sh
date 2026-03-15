#!/bin/bash
# Wrapper script to launch Esterian Conquest (DOS) from Enigma BBS
# Usage: run_ec_dos.sh <dropfile_path> <node_number> <server_port>

DROPFILE=$1
NODE=$2
PORT=$3
GAME_DIR="/home/mag/dev/esterian_conquest/original/v1.5"

echo "$(date) - Launching door with $@" >> /tmp/ec-door.log

# For ECBBS.EXE, the game client looks for DOOR.SYS or DORINFO1.DEF
# Enigma natively generates DOOR.SYS if we configure dropFileType: DOOR
# So we just need to ensure the copied file is in the root of C: mounted for the game

cp "$DROPFILE" "$GAME_DIR/DOOR.SYS"
chmod 666 "$GAME_DIR/DOOR.SYS"

export SDL_VIDEODRIVER=dummy

# DOSBox-X requires explicit socket connection if `io: socket` is used.
# The syntax for DOSBox-X serial port forwarding over TCP is usually:
# serial1=nullmodem server:127.0.0.1 port:$PORT
# Let's ensure we are calling ECGAME.EXE since ECBBS.EXE does NOT exist in original/v1.5/!

dosbox-x -conf /dev/null \
  -fastlaunch \
  -nogui \
  -c "serial1=nullmodem server:127.0.0.1 port:$PORT" \
  -c "mount c $GAME_DIR" \
  -c "c:" \
  -c "ECGAME.EXE /D:C:\\ /N:$NODE" \
  -c "exit" >> /tmp/ec-door.log 2>&1
