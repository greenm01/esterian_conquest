#!/bin/bash
export SDL_VIDEODRIVER=wayland
rm -f /tmp/starbase-ready/DOOR.SYS /tmp/starbase-ready/CHAIN.TXT /tmp/starbase-ready/DORINFO1.DEF
dosbox-x -defaultconf -nopromptfolder -set "dosv=off" -set "machine=vgaonly" -set "core=normal" -set "cputype=386_prefetch" -set "cycles=fixed 3000" -set "xms=false" -set "ems=false" -set "umb=false" -c "mount c /tmp/starbase-ready" -c "c:" -c "ECGAME" -c "pause"
