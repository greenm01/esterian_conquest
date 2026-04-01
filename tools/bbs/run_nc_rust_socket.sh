#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 3 ]; then
    echo "usage: $0 <game_dir> <dropfile_path> <srv_port> [extra nc-game args...]" >&2
    exit 64
fi

GAME_DIR=$1
DROPFILE=$2
SRV_PORT=$3
shift 3

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
RUST_DIR="$REPO_ROOT/rust"
RELEASE_BIN="$RUST_DIR/target/release/nc-game"
DEBUG_BIN="$RUST_DIR/target/debug/nc-game"

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

if [ -x "$RELEASE_BIN" ]; then
    GAME_CMD=("$RELEASE_BIN")
elif [ -x "$DEBUG_BIN" ]; then
    GAME_CMD=("$DEBUG_BIN")
else
    GAME_CMD=(cargo run -q -p nc-game --manifest-path "$RUST_DIR/Cargo.toml" --)
fi

GAME_ARGS=(
    --dir "$GAME_DIR"
    --dropfile "$DROPFILE"
    --encoding cp437
    --color-mode ansi16
    "$@"
)

shell_quote() {
    printf "%q " "$@"
}

SCRIPT_CMD="$(shell_quote "${GAME_CMD[@]}" "${GAME_ARGS[@]}")"

exec 3<>"/dev/tcp/127.0.0.1/$SRV_PORT"
exec script -qfc "$SCRIPT_CMD" /dev/null <&3 >&3
