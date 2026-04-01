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
SSH_USER_EXPLICIT=0
AUTH_KEYS_METHOD=""
AUTH_KEYS_PATH=""
AUTH_KEYS_EXPLICIT=0

usage() {
  cat <<'EOF'
Usage:
  ./scripts/start_local_gui_hosted_test.sh [options]

Options:
  --dir <path>         Hosted game directory. Default: /tmp/ec-player1-ui
  --relay <url>        Relay URL to publish to. Default: ws://localhost:8080
  --ssh-host <host>    SSH host published to players. Default: localhost
  --ssh-port <port>    SSH port published to players. Default: 22
  --ssh-user <user>    SSH user in gate config. Default: current user on localhost, ecgame otherwise
  --state-dir <path>   Temp gate config/identity dir. Default: /tmp/ec-local-gate
  --auth-keys-method <command|file>
                       Override SSH key provisioning mode. Requires --auth-keys-path.
  --auth-keys-path <path>
                       Override SSH key provisioning path. Requires --auth-keys-method.
  --help               Show this help

This helper starts a local nc-sysop nostr serve instance for a stress-test
game. It prints pending-seat invite codes when available and reports already
claimed seats for returning-player fixture testing.

Pending invite output looks like:

  nc-connect --join victim-sickness@localhost:8080

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
      SSH_USER_EXPLICIT=1
      shift 2
      ;;
    --state-dir)
      STATE_DIR="$2"
      shift 2
      ;;
    --auth-keys-method)
      AUTH_KEYS_METHOD="$2"
      AUTH_KEYS_EXPLICIT=1
      shift 2
      ;;
    --auth-keys-path)
      AUTH_KEYS_PATH="$2"
      AUTH_KEYS_EXPLICIT=1
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

GAME_DB="$GAME_DIR/ncgame.db"
CONFIG_PATH="$STATE_DIR/config.kdl"
IDENTITY_PATH="$STATE_DIR/identity.kdl"
EC_GAME_PATH="$RUST_DIR/target/debug/nc-game"

resolve_user_home() {
  getent passwd "$1" | awk -F: 'NR == 1 { print $6 }'
}

detect_authorized_keys_command() {
  local ssh_user="$1"
  local files=()
  if [[ -f /etc/ssh/sshd_config ]]; then
    files+=(/etc/ssh/sshd_config)
  fi
  local file
  for file in /etc/ssh/sshd_config.d/*.conf; do
    [[ -f "$file" ]] && files+=("$file")
  done
  [[ ${#files[@]} -gt 0 ]] || return 1

  awk -v user="$ssh_user" '
    BEGIN {
      in_match = 0
      found = 0
      global_command = ""
    }
    /^[[:space:]]*#/ || /^[[:space:]]*$/ { next }
    {
      line = $0
      sub(/^[[:space:]]+/, "", line)
      lower = tolower(line)
      if (lower ~ /^match[[:space:]]+/) {
        in_match = 0
        if (lower ~ ("^match[[:space:]]+user[[:space:]]+" tolower(user) "([[:space:]]|$)")) {
          in_match = 1
        }
        next
      }
      if (lower ~ /^authorizedkeyscommand[[:space:]]+/) {
        sub(/^[^[:space:]]+[[:space:]]+/, "", line)
        if (in_match) {
          found = 1
          print line
          exit
        }
        if (global_command == "") {
          global_command = line
        }
      }
    }
    END {
      if (!found && global_command != "") {
        print global_command
      }
    }
  ' "${files[@]}"
}

extract_gate_keys_dir() {
  local script_path="$1"
  sed -n 's/^KEY_DIR="\([^"]*\)"$/\1/p' "$script_path" | head -n 1
}

ensure_auth_keys_explicit_pair() {
  if [[ -n "$AUTH_KEYS_METHOD" && -z "$AUTH_KEYS_PATH" ]]; then
    echo "error: --auth-keys-method requires --auth-keys-path" >&2
    exit 1
  fi
  if [[ -z "$AUTH_KEYS_METHOD" && -n "$AUTH_KEYS_PATH" ]]; then
    echo "error: --auth-keys-path requires --auth-keys-method" >&2
    exit 1
  fi
}

fallback_to_current_user_file_mode() {
  local current_user="$1"
  local home
  home="$(resolve_user_home "$current_user")"
  if [[ -z "$home" ]]; then
    return 1
  fi
  SSH_USER="$current_user"
  AUTH_KEYS_METHOD="file"
  AUTH_KEYS_PATH="$home/.ssh/authorized_keys"
  return 0
}

is_loopback_host() {
  case "$1" in
    localhost|127.0.0.1|::1|"[::1]")
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

detect_auth_keys_config() {
  ensure_auth_keys_explicit_pair
  if [[ -n "$AUTH_KEYS_METHOD" ]]; then
    return 0
  fi

  if [[ $SSH_USER_EXPLICIT -eq 0 ]] && is_loopback_host "$SSH_HOST"; then
    local current_user
    current_user="$(id -un)"
    if fallback_to_current_user_file_mode "$current_user"; then
      return 0
    fi
  fi

  local command_line
  command_line="$(detect_authorized_keys_command "$SSH_USER" || true)"
  if [[ -n "$command_line" ]]; then
    local command_path
    command_path="$(printf '%s\n' "$command_line" | awk '{print $1}')"
    if [[ -n "$command_path" && -f "$command_path" ]]; then
      local key_dir
      key_dir="$(extract_gate_keys_dir "$command_path")"
      if [[ -n "$key_dir" && -d "$key_dir" && -w "$key_dir" ]]; then
        AUTH_KEYS_METHOD="command"
        AUTH_KEYS_PATH="$key_dir"
        return 0
      fi
    fi

    local current_user
    current_user="$(id -un)"
    if [[ $SSH_USER_EXPLICIT -eq 0 ]] && fallback_to_current_user_file_mode "$current_user"; then
      return 0
    fi

    echo "error: sshd uses AuthorizedKeysCommand for user '$SSH_USER', but the detected key store is not usable" >&2
    echo "pass a working --auth-keys-method/--auth-keys-path pair, or use a different --ssh-user" >&2
    exit 1
  fi

  local requested_user="$SSH_USER"
  local home
  home="$(resolve_user_home "$requested_user")"
  if [[ -n "$home" ]]; then
    AUTH_KEYS_METHOD="file"
    AUTH_KEYS_PATH="$home/.ssh/authorized_keys"
    return 0
  fi

  echo "error: could not determine how local sshd provisions keys for user '$requested_user'" >&2
  echo "pass --auth-keys-method and --auth-keys-path explicitly for your setup" >&2
  exit 1
}

if [[ ! -f "$GAME_DB" ]]; then
  echo "error: expected $GAME_DB" >&2
  exit 1
fi

PENDING_SEATS="$(
  sqlite3 "$GAME_DB" \
    "select player_record_index || '|' || invite_code from hosted_player_seats where claim_status = 'pending' order by player_record_index;"
)"
CLAIMED_SEATS="$(
  sqlite3 "$GAME_DB" \
    "select player_record_index || '|' || ifnull(player_npub, '') from hosted_player_seats where claim_status = 'claimed' order by player_record_index;"
)"

if [[ -z "$PENDING_SEATS" && -z "$CLAIMED_SEATS" ]]; then
  echo "error: no pending or claimed hosted seats found in $GAME_DB" >&2
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

detect_auth_keys_config

mkdir -p "$STATE_DIR"
if [[ "$AUTH_KEYS_METHOD" == "file" ]]; then
  mkdir -p "$(dirname "$AUTH_KEYS_PATH")"
  : > "$AUTH_KEYS_PATH"
fi

cat > "$CONFIG_PATH" <<EOF
relay "$RELAY_URL"
ssh-host "$SSH_HOST"
ssh-port $SSH_PORT
ssh-user "$SSH_USER"
nc-game-path "$EC_GAME_PATH"
auth-keys-method "$AUTH_KEYS_METHOD"
auth-keys-path "$AUTH_KEYS_PATH"
key-ttl $KEY_TTL
game "$GAME_DIR"
EOF

echo "Building nc-game and nc-sysop ..."
(cd "$RUST_DIR" && cargo build -q -p nc-game -p nc-sysop)

if [[ ! -f "$IDENTITY_PATH" ]]; then
  echo "Initializing local gate identity at $IDENTITY_PATH ..."
  (cd "$RUST_DIR" && cargo run -q -p nc-sysop -- nostr init --identity "$IDENTITY_PATH")
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
if [[ -n "$PENDING_SEATS" ]]; then
  echo "Pending invite commands:"
  while IFS='|' read -r seat invite; do
    [[ -n "$seat" ]] || continue
    echo "  Seat $seat"
    echo "    nc-connect --join ${invite}@${INVITE_SUFFIX}"
  done <<< "$PENDING_SEATS"
  echo
fi

if [[ -n "$CLAIMED_SEATS" ]]; then
  echo "Claimed seats:"
  while IFS='|' read -r seat npub; do
    [[ -n "$seat" ]] || continue
    echo "  Seat $seat"
    echo "    $npub"
  done <<< "$CLAIMED_SEATS"
  echo "Returning-player reconnects are available for the identities above."
  echo
fi

echo
echo "Starting local hosted daemon with:"
echo "  game dir:   $GAME_DIR"
echo "  relay:      $RELAY_URL"
echo "  ssh target: ${SSH_HOST}:${SSH_PORT}"
echo "  ssh user:   $SSH_USER"
echo "  auth mode:  $AUTH_KEYS_METHOD"
echo "  auth path:  $AUTH_KEYS_PATH"
echo "  state dir:  $STATE_DIR"
echo
echo "Leave this helper running while you test GUI invite joins."
echo

cd "$RUST_DIR"
exec cargo run -q -p nc-sysop -- nostr serve --config "$CONFIG_PATH" --identity "$IDENTITY_PATH"
