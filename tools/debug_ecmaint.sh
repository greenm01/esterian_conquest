#!/usr/bin/env bash
SCENARIO=/tmp/ecmaint-debug-token

env SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy \
dosbox-x \
  -defaultconf \
  -nopromptfolder \
  -nogui \
  -nomenu \
  -defaultdir "$SCENARIO" \
  -set "dosv=off" \
  -set "machine=vgaonly" \
  -set "core=normal" \
  -set "cputype=386_prefetch" \
  -set "cycles=fixed 3000" \
  -set "xms=false" \
  -set "ems=false" \
  -set "umb=false" \
  -set "output=surface" \
  -c "mount c $SCENARIO" \
  -c "c:" \
  -c "DEBUGBOX ECMAINT"
