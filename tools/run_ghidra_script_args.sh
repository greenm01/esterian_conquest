#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
source "$SCRIPT_DIR/ghidra_env.sh"

export XDG_CONFIG_HOME="/tmp/fake-xdg-config"
export XDG_CACHE_HOME="/tmp/fake-xdg-cache"

ANALYZE_HEADLESS=$(resolve_analyze_headless "$REPO_ROOT")

PROJECT=$1
SCRIPT=$2
SOURCE_DIR="$REPO_ROOT/tools/ghidra_scripts"
TMP_SCRIPT_DIR="$REPO_ROOT/tools/ghidra_scripts_tmp"

mkdir -p "$TMP_SCRIPT_DIR"
if [[ -f "$SOURCE_DIR/$SCRIPT" ]]; then
  cp "$SOURCE_DIR/$SCRIPT" "$TMP_SCRIPT_DIR/$SCRIPT"
fi

"$ANALYZE_HEADLESS" \
  "$REPO_ROOT/.ghidra/projects" \
  "$PROJECT" \
  -process MEMDUMP.BIN \
  -noanalysis \
  -scriptPath "$TMP_SCRIPT_DIR" \
  -postScript "$SCRIPT"
