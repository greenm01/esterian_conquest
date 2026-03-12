#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

PROJECT_NAME="${1:-ecmaint-live}"
SCRIPT_NAME="${2:-ECMaintTokenAnchors.java}"

GHIDRA_HOME=$(ls -1d "$HOME"/tools/ghidra_*_PUBLIC 2>/dev/null | sort -V | tail -n 1)

export XDG_CONFIG_HOME="$REPO_ROOT/.ghidra/xdg-config"
export XDG_CACHE_HOME="$REPO_ROOT/.ghidra/xdg-cache"

"$GHIDRA_HOME/support/analyzeHeadless" \
  "$REPO_ROOT/.ghidra/projects" \
  "$PROJECT_NAME" \
  -process MEMDUMP.BIN \
  -noanalysis \
  -scriptPath "$REPO_ROOT/tools/ghidra_scripts" \
  -postScript "$SCRIPT_NAME" "$REPO_ROOT/artifacts/ghidra/ecmaint-live"
