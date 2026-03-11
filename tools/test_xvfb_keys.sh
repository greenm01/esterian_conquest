#!/bin/bash
Xvfb :98 -screen 0 640x480x24 &
XVFB_PID=$!
sleep 1
DISPLAY=:98 dosbox-x -defaultconf -nopromptfolder -defaultdir /tmp/starbase-ready -set "dosv=off" -set "machine=vgaonly" -set "core=normal" -set "cputype=386_prefetch" -set "cycles=fixed 3000" -c "mount c /tmp/starbase-ready" -c "c:" -c "ECGAME" -c "exit" &
DOSBOX_PID=$!
sleep 8
DISPLAY=:98 xdotool search --class dosbox windowactivate --sync key --delay 400 p a y g g q
sleep 3
kill -9 $DOSBOX_PID
kill -9 $XVFB_PID
rm -f /tmp/.X98-lock
