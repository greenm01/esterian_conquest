#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/../../.." && pwd)
source "$REPO_ROOT/tools/ghidra_env.sh"

# Use a temporary project directory in the sandbox
PROJECT_DIR="$SCRIPT_DIR/ghidra_project"
mkdir -p "$PROJECT_DIR"

ANALYZE_HEADLESS=$(resolve_analyze_headless "$REPO_ROOT")

echo "Running headless Ghidra analysis on ECGAMEU.EXE..."
"$ANALYZE_HEADLESS" \
    "$PROJECT_DIR" \
    "gemini_sandbox" \
    -import "$SCRIPT_DIR/ECGAMEU.EXE" \
    -overwrite \
    -scriptPath "$SCRIPT_DIR" \
    -postScript DumpEntry.java \
    2>&1 | tee "$SCRIPT_DIR/analysis_log.txt"
