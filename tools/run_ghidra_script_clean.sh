#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
source "$SCRIPT_DIR/ghidra_env.sh"

export XDG_CONFIG_HOME="/tmp/fake-xdg-config"
export XDG_CACHE_HOME="/tmp/fake-xdg-cache"

ANALYZE_HEADLESS=$(resolve_analyze_headless "$REPO_ROOT")

"$ANALYZE_HEADLESS" \
  "$REPO_ROOT/.ghidra/projects" \
  ecmaint-live \
  -process MEMDUMP.BIN \
  -noanalysis \
  -scriptPath "$REPO_ROOT/tools/ghidra_scripts_tmp" \
  -postScript AnalyzeCaller.java
