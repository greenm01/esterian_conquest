#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
export XDG_CONFIG_HOME="/tmp/fake-xdg-config"
export XDG_CACHE_HOME="/tmp/fake-xdg-cache"
GHIDRA_HOME=$(ls -1d "$HOME"/tools/ghidra_*_PUBLIC 2>/dev/null | sort -V | tail -n 1)

"$GHIDRA_HOME/support/analyzeHeadless" \
  "$REPO_ROOT/.ghidra/projects" \
  "ecmaint-live" \
  -process MEMDUMP.BIN \
  -scriptPath "$REPO_ROOT/tools/ghidra_scripts_tmp" \
  -postScript FindRefsByAddr.java
