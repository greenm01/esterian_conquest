#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 2 ]; then
    echo "usage: $0 <game_dir> <dropfile_path> [extra ec-game args...]" >&2
    exit 64
fi

GAME_DIR=$1
DROPFILE=$2
shift 2

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
RUST_DIR="$REPO_ROOT/rust"
RELEASE_BIN="$RUST_DIR/target/release/ec-game"
DEBUG_BIN="$RUST_DIR/target/debug/ec-game"

if [ ! -d "$GAME_DIR" ]; then
    echo "ec-game launcher error: game dir not found: $GAME_DIR" >&2
    exit 66
fi

if [ ! -f "$DROPFILE" ]; then
    echo "ec-game launcher error: dropfile not found: $DROPFILE" >&2
    exit 66
fi

if [ -z "${EC_CLIENT_EXPORT_ROOT:-}" ]; then
    export EC_CLIENT_EXPORT_ROOT="$GAME_DIR/exports"
fi
mkdir -p "$EC_CLIENT_EXPORT_ROOT"

if [ -n "${EC_CLIENT_QUEUE_DIR:-}" ]; then
    mkdir -p "$EC_CLIENT_QUEUE_DIR"
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
exec cargo run -q -p ec-game -- "${COMMON_ARGS[@]}" "$@"
