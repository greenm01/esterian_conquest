#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
setup_script="$repo_root/scripts/setup_classic_probe_game.py"
default_target="/tmp/ec-classic-report-probe"

target_dir="$default_target"
if [ $# -gt 0 ] && [[ "$1" != -* ]]; then
  target_dir=$1
  shift
fi

exec python3 "$setup_script" "$target_dir" --force "$@"
