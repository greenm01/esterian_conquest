#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
source "$SCRIPT_DIR/ghidra_env.sh"
ANALYZE_HEADLESS=$(resolve_analyze_headless "$REPO_ROOT")

export XDG_CONFIG_HOME="$REPO_ROOT/.ghidra/xdg-config"
export XDG_CACHE_HOME="$REPO_ROOT/.ghidra/xdg-cache"

"$ANALYZE_HEADLESS" \
  "$REPO_ROOT/.ghidra/projects" \
  ecmaint-live \
  -process MEMDUMP.BIN \
  -noanalysis \
  -scriptPath "$REPO_ROOT/tools/ghidra_scripts" \
  -postScript ECMaintTokenCallerReport.java
