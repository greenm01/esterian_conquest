#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
source "$SCRIPT_DIR/ghidra_env.sh"

PROJECT_NAME="${1:-ecmaint-live}"
SCRIPT_NAME="${2:-ECMaintTokenAnchors.java}"
SOURCE_DIR="$REPO_ROOT/tools/ghidra_scripts"
TMP_SCRIPT_DIR="$REPO_ROOT/tools/ghidra_scripts_tmp"

ANALYZE_HEADLESS=$(resolve_analyze_headless "$REPO_ROOT")

export XDG_CONFIG_HOME="$REPO_ROOT/.ghidra/xdg-config"
export XDG_CACHE_HOME="$REPO_ROOT/.ghidra/xdg-cache"

mkdir -p "$TMP_SCRIPT_DIR"
if [[ -f "$SOURCE_DIR/$SCRIPT_NAME" ]]; then
  cp "$SOURCE_DIR/$SCRIPT_NAME" "$TMP_SCRIPT_DIR/$SCRIPT_NAME"
fi

"$ANALYZE_HEADLESS" \
  "$REPO_ROOT/.ghidra/projects" \
  "$PROJECT_NAME" \
  -process MEMDUMP.BIN \
  -noanalysis \
  -scriptPath "$TMP_SCRIPT_DIR" \
  -postScript "$SCRIPT_NAME" "$REPO_ROOT/artifacts/ghidra/ecmaint-live"
