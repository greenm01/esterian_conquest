#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
  echo "usage: $0 <game_dir> [player_number]" >&2
  exit 2
fi

game_dir=$1
player_number=${2:-1}
mount_dir=$game_dir
temp_link=

if [ ! -d "$game_dir" ]; then
  echo "game directory not found: $game_dir" >&2
  exit 1
fi

if [ ! -f "$game_dir/ECGAME.EXE" ] && [ ! -f "$game_dir/ECGAME" ]; then
  echo "ECGAME executable not found in: $game_dir" >&2
  exit 1
fi

if [[ "$game_dir" == *" "* ]]; then
  temp_link="/tmp/ecgame-run-${USER:-user}-$$"
  ln -sfn "$game_dir" "$temp_link"
  mount_dir=$temp_link
fi

trap 'if [ -n "${temp_link:-}" ]; then rm -f "$temp_link"; fi' EXIT

GAME_DIR="$game_dir" PLAYER_NUMBER="$player_number" python3 - <<'PY'
import os
from pathlib import Path
from tools.ecgame_dropfiles import write_chain_txt

write_chain_txt(Path(os.environ["GAME_DIR"]) / "CHAIN.TXT", player_number=int(os.environ["PLAYER_NUMBER"]))
PY

export SDL_VIDEODRIVER="${SDL_VIDEODRIVER_OVERRIDE:-wayland}"
export SDL_AUDIODRIVER="${SDL_AUDIODRIVER_OVERRIDE:-dummy}"

dosbox-x \
  -defaultconf \
  -nopromptfolder \
  -set "dosv=off" \
  -set "machine=vgaonly" \
  -set "core=normal" \
  -set "cputype=386_prefetch" \
  -set "cycles=fixed 3000" \
  -set "xms=false" \
  -set "ems=false" \
  -set "umb=false" \
  -set "output=surface" \
  -c "mount c $mount_dir" \
  -c "c:" \
  -c "mode co80" \
  -c "ECGAME"
