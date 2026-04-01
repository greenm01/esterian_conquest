#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 2 ]; then
    echo "usage: $0 <game_dir> <dropfile_path> [extra nc-game args...]" >&2
    exit 64
fi

GAME_DIR=$1
DROPFILE=$2
shift 2

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
RUST_DIR="$REPO_ROOT/rust"
RELEASE_BIN="$RUST_DIR/target/release/nc-game"
DEBUG_BIN="$RUST_DIR/target/debug/nc-game"
TRACE_TRIGGER=/tmp/nc-door-trace.trigger

if [ ! -d "$GAME_DIR" ]; then
    echo "nc-game launcher error: game dir not found: $GAME_DIR" >&2
    exit 66
fi

if [ ! -f "$DROPFILE" ]; then
    echo "nc-game launcher error: dropfile not found: $DROPFILE" >&2
    exit 66
fi

if [ -z "${NC_CLIENT_EXPORT_ROOT:-}" ]; then
    export NC_CLIENT_EXPORT_ROOT="$GAME_DIR/exports"
fi
mkdir -p "$NC_CLIENT_EXPORT_ROOT"

if [ -n "${NC_CLIENT_QUEUE_DIR:-}" ]; then
    mkdir -p "$NC_CLIENT_QUEUE_DIR"
fi

if [ -z "${NC_GAME_DOOR_TRACE_DIR:-}" ] && [ -f "$TRACE_TRIGGER" ]; then
    NC_GAME_DOOR_TRACE_DIR=$(head -n 1 "$TRACE_TRIGGER" | tr -d '\r')
    if [ -n "$NC_GAME_DOOR_TRACE_DIR" ]; then
        export NC_GAME_DOOR_TRACE_DIR
    fi
fi

COMMON_ARGS=(
    --dir "$GAME_DIR"
    --dropfile "$DROPFILE"
    --encoding cp437
    --color-mode ansi16
)

if [ -x "$RELEASE_BIN" ]; then
    exec "$RELEASE_BIN" "${COMMON_ARGS[@]}" "$@"
fi

if [ -x "$DEBUG_BIN" ]; then
    exec "$DEBUG_BIN" "${COMMON_ARGS[@]}" "$@"
fi

cd "$RUST_DIR"
exec cargo run -q -p nc-game -- "${COMMON_ARGS[@]}" "$@"
