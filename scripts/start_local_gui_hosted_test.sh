#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_DIR="$REPO_ROOT/rust"

GAME_DIR="/tmp/ec-player1-ui"
RELAY_URL="ws://localhost:8080"
SSH_HOST="localhost"
SSH_PORT="22"
SSH_USER="ecgame"
STATE_DIR="/tmp/ec-local-gate"
KEY_TTL="60"

usage() {
  cat <<'EOF'
Usage:
  ./scripts/start_local_gui_hosted_test.sh [options]

Options:
  --dir <path>         Hosted game directory. Default: /tmp/ec-player1-ui
  --relay <url>        Relay URL to publish to. Default: ws://localhost:8080
  --ssh-host <host>    SSH host published to players. Default: localhost
  --ssh-port <port>    SSH port published to players. Default: 22
  --ssh-user <user>    SSH user in gate config. Default: ecgame
  --state-dir <path>   Temp gate config/identity dir. Default: /tmp/ec-local-gate
  --help               Show this help

This helper starts a local ec-sysop nostr serve instance for a stress-test
game and prints full invite codes like:

  ec-connect --join victim-sickness@localhost:8080

Prerequisite:
  A relay must already be listening at the chosen --relay URL.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dir)
      GAME_DIR="$2"
      shift 2
      ;;
    --relay)
      RELAY_URL="$2"
      shift 2
      ;;
    --ssh-host)
      SSH_HOST="$2"
      shift 2
      ;;
    --ssh-port)
      SSH_PORT="$2"
      shift 2
      ;;
    --ssh-user)
      SSH_USER="$2"
      shift 2
      ;;
    --state-dir)
      STATE_DIR="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command: $1" >&2
    exit 1
  fi
}

require_cmd cargo
require_cmd sqlite3
require_cmd python3

GAME_DB="$GAME_DIR/ecgame.db"
CONFIG_PATH="$STATE_DIR/config.kdl"
IDENTITY_PATH="$STATE_DIR/identity.kdl"
AUTH_KEYS_PATH="$STATE_DIR/authorized_keys"
EC_GAME_PATH="$RUST_DIR/target/debug/ec-game"

if [[ ! -f "$GAME_DB" ]]; then
  echo "error: expected $GAME_DB" >&2
  exit 1
fi

PENDING_SEATS="$(
  sqlite3 "$GAME_DB" \
    "select player_record_index || '|' || invite_code from hosted_player_seats where claim_status = 'pending' order by player_record_index;"
)"

if [[ -z "$PENDING_SEATS" ]]; then
  echo "error: no pending hosted seats found in $GAME_DB" >&2
  exit 1
fi

python3 - "$RELAY_URL" <<'PY'
import socket
import sys
import urllib.parse

relay = sys.argv[1]
parsed = urllib.parse.urlparse(relay)
if parsed.scheme not in {"ws", "wss"} or not parsed.hostname:
    raise SystemExit(f"error: unsupported relay URL: {relay}")
port = parsed.port or (443 if parsed.scheme == "wss" else 80)
try:
    with socket.create_connection((parsed.hostname, port), timeout=1.5):
        pass
except OSError as exc:
    raise SystemExit(
        f"error: relay {relay} is not reachable: {exc}\n"
        "start your local relay first, then re-run this helper"
    )
PY

mkdir -p "$STATE_DIR"
: > "$AUTH_KEYS_PATH"

cat > "$CONFIG_PATH" <<EOF
relay "$RELAY_URL"
ssh-host "$SSH_HOST"
ssh-port $SSH_PORT
ssh-user "$SSH_USER"
ec-game-path "$EC_GAME_PATH"
auth-keys-method "file"
auth-keys-path "$AUTH_KEYS_PATH"
key-ttl $KEY_TTL
game "$GAME_DIR"
EOF

echo "Building ec-game and ec-sysop ..."
(cd "$RUST_DIR" && cargo build -q -p ec-game -p ec-sysop)

if [[ ! -f "$IDENTITY_PATH" ]]; then
  echo "Initializing local gate identity at $IDENTITY_PATH ..."
  (cd "$RUST_DIR" && cargo run -q -p ec-sysop -- nostr init --identity "$IDENTITY_PATH")
fi

INVITE_SUFFIX="$(
  python3 - "$RELAY_URL" <<'PY'
import sys
import urllib.parse

parsed = urllib.parse.urlparse(sys.argv[1])
host = parsed.hostname or ""
port = parsed.port
if not host:
    raise SystemExit("error: relay host missing")
if port is None:
    print(host)
else:
    print(f"{host}:{port}")
PY
)"

echo
echo "Pending invite commands:"
while IFS='|' read -r seat invite; do
  [[ -n "$seat" ]] || continue
  echo "  Seat $seat"
  echo "    ec-connect --join ${invite}@${INVITE_SUFFIX}"
done <<< "$PENDING_SEATS"

echo
echo "Starting local hosted daemon with:"
echo "  game dir:   $GAME_DIR"
echo "  relay:      $RELAY_URL"
echo "  ssh target: ${SSH_HOST}:${SSH_PORT}"
echo "  state dir:  $STATE_DIR"
echo
echo "Leave this helper running while you test GUI invite joins."
echo

cd "$RUST_DIR"
exec cargo run -q -p ec-sysop -- nostr serve --config "$CONFIG_PATH" --identity "$IDENTITY_PATH"
