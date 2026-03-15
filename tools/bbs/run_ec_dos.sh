#!/bin/bash
# Wrapper script to launch Esterian Conquest (DOS) from Enigma BBS
# Usage: run_ec_dos.sh <dropfile_path> <node_number>

DROPFILE=$1
NODE=$2
GAME_DIR="/home/mag/dev/esterian_conquest/original/v1.5"

echo "$(date) - Launching door with $@" >> /tmp/ec-door.log

# Read the docs carefully - ECGAME prefers CHAIN.TXT with *local* console parameters 
# for local play, OR standard remote DOOR.SYS / DORINFO for actual BBS door routing.

# Enigma will natively create DOOR.SYS
cp "$DROPFILE" "$GAME_DIR/DOOR.SYS"

export SDL_VIDEODRIVER=dummy

# DOSBox-X requires specific settings to forward DOS INT 14h / FOSSIL / Stdio
# When Enigma uses `io: stdio`, we must ensure DOSBox-X is routing output properly.

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
