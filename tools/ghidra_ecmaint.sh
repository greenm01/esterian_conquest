#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  tools/ghidra_ecmaint.sh [--overwrite] [--project NAME] [--analysis-timeout SECONDS] [binary]

Defaults:
  binary   = original/v1.5/ECMAINT.EXE
  project  = ec-v15
  timeout  = no per-file timeout

Environment:
  GHIDRA_HOME    Path to the extracted Ghidra directory.
                 If unset, the script also checks:
                 - /usr/share/ghidra
                 - ./ghidra
                 - the newest $HOME/tools/ghidra_*_PUBLIC directory
EOF
}

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

PROJECT_NAME="ec-v15"
OVERWRITE=0
TARGET_BINARY=""
ANALYSIS_TIMEOUT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --overwrite)
      OVERWRITE=1
      shift
      ;;
    --project)
      if [[ $# -lt 2 ]]; then
        echo "missing value for --project" >&2
        usage >&2
        exit 2
      fi
      PROJECT_NAME=$2
      shift 2
      ;;
    --analysis-timeout)
      if [[ $# -lt 2 ]]; then
        echo "missing value for --analysis-timeout" >&2
        usage >&2
        exit 2
      fi
      ANALYSIS_TIMEOUT=$2
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    -*)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      if [[ -n "$TARGET_BINARY" ]]; then
        echo "only one binary path may be provided" >&2
        usage >&2
        exit 2
      fi
      TARGET_BINARY=$1
      shift
      ;;
  esac
done

if [[ -z "$TARGET_BINARY" ]]; then
  TARGET_BINARY="$REPO_ROOT/original/v1.5/ECMAINT.EXE"
fi

if [[ ! -f "$TARGET_BINARY" ]]; then
  echo "binary not found: $TARGET_BINARY" >&2
  exit 1
fi

find_ghidra_home() {
  if [[ -n "${GHIDRA_HOME:-}" && -x "${GHIDRA_HOME}/support/analyzeHeadless" ]]; then
    printf '%s\n' "$GHIDRA_HOME"
    return 0
  fi

  if [[ -x "/usr/share/ghidra/support/analyzeHeadless" ]]; then
    printf '%s\n' "/usr/share/ghidra"
    return 0
  fi

  if [[ -x "$REPO_ROOT/ghidra/support/analyzeHeadless" ]]; then
    printf '%s\n' "$REPO_ROOT/ghidra"
    return 0
  fi

  local candidate
  for candidate in "$HOME"/tools/ghidra_*_PUBLIC; do
    if [[ -x "$candidate/support/analyzeHeadless" ]]; then
      printf '%s\n' "$candidate"
    fi
  done | sort -V | tail -n 1 | grep -q . && {
    for candidate in "$HOME"/tools/ghidra_*_PUBLIC; do
      if [[ -x "$candidate/support/analyzeHeadless" ]]; then
        printf '%s\n' "$candidate"
      fi
    done | sort -V | tail -n 1
    return 0
  }

  return 1
}

if ! GHIDRA_HOME=$(find_ghidra_home); then
  cat >&2 <<'EOF'
Could not find Ghidra.

Set GHIDRA_HOME to an extracted Ghidra directory, for example:
  export GHIDRA_HOME="$HOME/tools/ghidra_12.0.2_PUBLIC"
EOF
  exit 1
fi

ANALYZE_HEADLESS="$GHIDRA_HOME/support/analyzeHeadless"
PROJECT_ROOT="$REPO_ROOT/.ghidra/projects"
OUTPUT_ROOT="$REPO_ROOT/artifacts/ghidra/ecmaint"
LOG_FILE="$OUTPUT_ROOT/analyze.log"
XDG_CONFIG_ROOT="$REPO_ROOT/.ghidra/xdg-config"
XDG_CACHE_ROOT="$REPO_ROOT/.ghidra/xdg-cache"

mkdir -p "$PROJECT_ROOT" "$OUTPUT_ROOT" "$XDG_CONFIG_ROOT" "$XDG_CACHE_ROOT"

cmd=(
  "$ANALYZE_HEADLESS"
  "$PROJECT_ROOT"
  "$PROJECT_NAME"
  -import "$TARGET_BINARY"
  -log "$LOG_FILE"
  -scriptlog "$OUTPUT_ROOT/script.log"
)

if [[ -n "$ANALYSIS_TIMEOUT" ]]; then
  cmd+=(-analysisTimeoutPerFile "$ANALYSIS_TIMEOUT")
fi

if [[ $OVERWRITE -eq 1 ]]; then
  cmd+=(-overwrite)
fi

echo "Ghidra home:   $GHIDRA_HOME"
echo "Project root:  $PROJECT_ROOT"
echo "Project name:  $PROJECT_NAME"
echo "Target binary: $TARGET_BINARY"
echo "Log file:      $LOG_FILE"
echo "XDG config:    $XDG_CONFIG_ROOT"
echo "XDG cache:     $XDG_CACHE_ROOT"

XDG_CONFIG_HOME="$XDG_CONFIG_ROOT" \
XDG_CACHE_HOME="$XDG_CACHE_ROOT" \
"${cmd[@]}"
