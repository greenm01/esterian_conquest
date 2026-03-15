#!/bin/bash
# Wrapper script to launch Esterian Conquest (DOS) from Enigma BBS
# Usage: run_ec_dos.sh <dropfile_path> <node_number> <port>

DROPFILE=$1
NODE=$2
PORT=$3

# Find the repository root dynamically
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
GAME_DIR="$REPO_ROOT/original/v1.5"
LOGFILE="/tmp/ec-door.log"

echo "$(date) - Launching door with $@" >> $LOGFILE

# Create a Python script to convert/generate the strict CHAIN.TXT format
cat << 'PY' > /tmp/convert_to_chain.py
import sys
import os

repo_root = sys.argv[1]
out_path = sys.argv[2]

sys.path.insert(0, repo_root)
from tools.ecgame_dropfiles import write_chain_txt

# Set to remote modem defaults to ensure the COM port triggers
write_chain_txt(out_path, remote=1, com_port=1, user_baud=115200, com_baud=115200)
PY

# Generate CHAIN.TXT directly into the game directory
python3 /tmp/convert_to_chain.py "$REPO_ROOT" "$GAME_DIR/CHAIN.TXT"

# Headless mode for SDL to prevent Wayland/X11 crashes
export SDL_VIDEODRIVER=dummy

# ECGAME is launched with ZERO arguments directly from its directory
cat << 'BAT' > "$GAME_DIR/RUN.BAT"
@ECHO OFF
C:
ECGAME.EXE
exit
BAT

# Generate a temporary dynamic dosbox.conf for this node
CONF_FILE="/tmp/ec_dosbox_node${NODE}.conf"
cat << CONF > "$CONF_FILE"
[sdl]
output=dummy

[dosv]
dosv=off

[cpu]
core=normal
cputype=386_prefetch
# NOTE: Using 'cycles=max' instead of 'fixed 3000' is CRITICAL! 
# Low cycles severely bottleneck the virtual UART/COM rendering over Telnet,
# making the door feel like a 300 baud modem.
cycles=max

[machine]
machine=vgaonly

[serial]
# Telnet handoff from Enigma via nullmodem socket
serial1=nullmodem server:127.0.0.1 port:$PORT transparent:1 telnet:1

[autoexec]
mount c $GAME_DIR
c:
RUN.BAT
CONF

echo "Running dosbox-x..." >> $LOGFILE
dosbox-x -conf "$CONF_FILE" -fastlaunch -nogui >> $LOGFILE 2>&1
echo "dosbox-x exited with $?" >> $LOGFILE
