# Enigma BBS Legacy DOS Door Setup

This guide covers the legacy compatibility path: running the original DOS
`ECGAME.EXE` as a door under [Enigma BBS](https://github.com/NuSkooler/enigma-bbs)
through `dosbox-x`.

This is not the primary EC deployment story. The main direction of the
project is the Rust-native stack:

- `ec-sysop` for campaign setup and maintenance
- `ec-game` as the player-facing client

Use this guide when you explicitly want to host the original DOS client for
compatibility or migration reasons.

## The Dropfile Blocker & Lessons Learned

Setting up `ECGAME.EXE` v1.5 under modern BBS systems comes with several harsh parser restrictions that are largely undocumented or contradictory in the original manuals.

**Lessons Learned:**
1. **Dropfile Formatting:** `ECGAME.EXE` is notoriously strict about its dropfile. While the docs claim it supports `DOOR.SYS` or `DORINFO`, modern generated versions of these files will cause the game to immediately crash/exit back to DOS with an `0x1C` exit code (often logging `unexpected End Of File` in the game's internal `ERRORS.TXT`). 
2. **The `CHAIN.TXT` Solution:** The most reliable dropfile format to use is a 32-line WWIV-style `CHAIN.TXT`. It must have precise field counts and strict `\r\n` (CRLF) DOS line endings.
3. **Execution Path:** Do **not** use the `/L` flag (which `ECGAME` actually treats as a path to a dropfile, not a "local" flag). Do **not** pass explicit paths like `ECGAME.EXE C:\DROP`. Instead, place `CHAIN.TXT` directly into the mounted game directory (`C:\`) and launch `ECGAME.EXE` with zero arguments. It will auto-detect the file.
4. **Wayland / Headless Servers:** Running DOSBox-X headlessly via an SSH/Telnet server (especially on Wayland) will crash unless you forcefully disable SDL window creation. This is solved by exporting `SDL_VIDEODRIVER=dummy` and using the `-nogui` flag.

## 1. The Wrapper Script

Because Enigma's native dropfiles will crash the game, we use a wrapper script that ignores the Enigma dropfile, intercepts the socket handoff, dynamically generates a strict `CHAIN.TXT` using our Python helper, and launches `dosbox-x`.

Create or use the existing wrapper script (`tools/bbs/run_ec_dos.sh`):

```bash
#!/bin/bash
# Wrapper script to launch Esterian Conquest (DOS) from Enigma BBS
# Usage: run_ec_dos.sh <dropfile_path> <node_number> <port>

DROPFILE=$1
NODE=$2
PORT=$3
GAME_DIR="/path/to/esterian_conquest/original/v1.5"
LOGFILE="/tmp/ec-door.log"

echo "$(date) - Launching door with $@" >> $LOGFILE

# Create a Python script to convert/generate the strict CHAIN.TXT format
cat << 'PY' > /tmp/convert_to_chain.py
import sys
import os

# Use the project's utility to generate a strict 32-line WWIV CHAIN.TXT
sys.path.insert(0, '/path/to/esterian_conquest')
from tools.ecgame_dropfiles import write_chain_txt

# Set to remote modem defaults to ensure the COM port triggers
out_path = sys.argv[1]
write_chain_txt(out_path, remote=1, com_port=1, user_baud=115200, com_baud=115200)
PY

# Generate CHAIN.TXT directly into the game directory
python3 /tmp/convert_to_chain.py "$GAME_DIR/CHAIN.TXT"

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

# Launch DOSBox-X in headless mode
dosbox-x -conf "$CONF_FILE" -fastlaunch -nogui >> $LOGFILE 2>&1
```

> **Note on Speed:** Older documentation might suggest `cycles=fixed 3000`. This will throttle the game severely, causing it to render text over the COM port extremely slowly (like emulating a real dial-up modem). Setting `cycles=max` unleashes the emulated CPU, allowing it to blast text down the serial pipe at near-native Telnet speed.

Make sure the script is executable:
```bash
chmod +x tools/bbs/run_ec_dos.sh
```

## 2. Enigma BBS Configuration

In your Enigma BBS configuration (e.g. `config/menus/niltempus-doors.hjson`), you will use the `abracadabra` module to execute the local wrapper script. The `{srvPort}` argument automatically passes the socket port required for the nullmodem Telnet handoff.

```hjson
        //
        //  Esterian Conquest Door
        //
        doorEsterianConquest: {
            desc: Esterian Conquest
            module: abracadabra
            config: {
                name: Esterian Conquest
                dropFileType: DORINFO
                cmd: /path/to/esterian_conquest/tools/bbs/run_ec_dos.sh
                args: [
                    "{dropFile}",
                    "{node}",
                    "{srvPort}"
                ]
                io: socket
            }
        }
```

Add this menu action to a submit array somewhere in your system:
```hjson
                {
                    value: { command: "EC" }
                    action: @menu:doorEsterianConquest
                }
```

## 3. Performance Expectations: DOS vs. Rust EC

When testing the game through DOSBox-X via Enigma BBS, you will likely notice that the text "paints" or scrolls down the screen line-by-line, very reminiscent of a classic 90s dial-up modem connection. 

This happens because:
- **Legacy Serial Rendering:** `ECGAME.EXE` is hardcoded to render text character-by-character to a specific hardware address (the 16550 UART COM port).
- **Emulation Bottlenecks:** DOSBox-X is faithfully emulating this hardware stack, including its limitations. It forces the text through legacy DOS interrupts and bounds it by the virtual baud rate we set in `CHAIN.TXT` (115200). Even when using `cycles=max` to un-throttle the CPU, the serial I/O emulation remains a bottleneck.

**The Rust-native path is radically faster.**

The ongoing Rust EC stack bypasses this legacy architecture entirely.
Instead of simulating virtual hardware and pacing I/O via DOS interrupts, the
native client/server path can write full strings and buffers directly to modern
terminals and sockets. The DOS door path is therefore a compatibility option,
not the performance target.

## 4. Compatibility Notes

Currently, the `convert_to_chain.py` script hardcodes the user info (for
example `Sysop`). To fully integrate the legacy DOS door, you can parse the
`{dropFile}` that Enigma generates, extract the real caller alias/name, and
pass those values to `write_chain_txt(..., alias=real_alias, real_name=real_name)`.
