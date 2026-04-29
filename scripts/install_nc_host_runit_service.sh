#!/usr/bin/env sh
set -eu

REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
RUST_DIR="$REPO_ROOT/rust"
HOST_BIN="$RUST_DIR/target/debug/nc-host"

SERVICE_NAME="nc-host"
RUN_AS_USER=${SUDO_USER:-$(id -un)}
RELAY_URL="ws://localhost:8080"
INVITE_RELAY_HOST="localhost:8080"
GAME_ID="local-dev"
GAME_NAME="Local Dev NC"
PLAYERS="4"
TIER="sandbox"
START_SERVICE=1

usage() {
  cat <<'EOF'
Usage:
  ./scripts/install_nc_host_runit_service.sh [options]

Options:
  --user <name>                User account that runs nc-host. Default: current user.
  --games-root <path>          Hosted games root. Default: ~/.local/share/nc-host/games.
  --relay-url <url>            Relay URL. Default: ws://localhost:8080
  --invite-relay-host <host>   Invite relay host[:port]. Default: localhost:8080
  --game-id <slug>             Dev game directory name. Default: local-dev
  --game-name <name>           Dev game display name. Default: Local Dev NC
  --players <n>                Dev game player count. Default: 4
  --tier <sandbox|league>      Dev game tier. Default: sandbox
  --skip-game                  Install service without creating/opening a dev game.
  --no-start                   Install service without starting it.
  --help                       Show this help.

Void/runit dev installer for the localhost nc-host + Chorus relay lab.
It writes user config under ~/.config/nc-host, data under ~/.local/share/nc-host,
and installs /etc/sv/nc-host as a runit service.
EOF
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command: $1" >&2
    exit 1
  fi
}

shell_quote() {
  escaped=$(printf "%s" "$1" | sed "s/'/'\\\\''/g")
  printf "'%s'" "$escaped"
}

run_root() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  else
    sudo "$@"
  fi
}

run_as_target() {
  if [ "$(id -un)" = "$RUN_AS_USER" ]; then
    "$@"
  elif [ "$(id -u)" -eq 0 ]; then
    su -s /bin/sh "$RUN_AS_USER" -c "$(quote_command "$@")"
  else
    sudo -u "$RUN_AS_USER" "$@"
  fi
}

quote_command() {
  quoted=""
  for arg do
    escaped=$(printf "%s" "$arg" | sed "s/'/'\\\\''/g")
    quoted="$quoted '${escaped}'"
  done
  printf "%s" "$quoted"
}

CREATE_GAME=1
GAMES_ROOT=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --user)
      RUN_AS_USER=$2
      shift 2
      ;;
    --games-root)
      GAMES_ROOT=$2
      shift 2
      ;;
    --relay-url)
      RELAY_URL=$2
      shift 2
      ;;
    --invite-relay-host)
      INVITE_RELAY_HOST=$2
      shift 2
      ;;
    --game-id)
      GAME_ID=$2
      shift 2
      ;;
    --game-name)
      GAME_NAME=$2
      shift 2
      ;;
    --players)
      PLAYERS=$2
      shift 2
      ;;
    --tier)
      TIER=$2
      shift 2
      ;;
    --skip-game)
      CREATE_GAME=0
      shift
      ;;
    --no-start)
      START_SERVICE=0
      shift
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

require_cmd cargo
require_cmd chpst
require_cmd cut
require_cmd getent
require_cmd grep
require_cmd install
require_cmd mktemp
require_cmd sed
require_cmd su
require_cmd sv
require_cmd svlogd
if [ "$(id -u)" -ne 0 ]; then
  require_cmd sudo
fi

TARGET_HOME=$(getent passwd "$RUN_AS_USER" | cut -d: -f6)
if [ -z "$TARGET_HOME" ]; then
  echo "error: could not resolve home directory for user $RUN_AS_USER" >&2
  exit 1
fi

CONFIG_ROOT="${XDG_CONFIG_HOME:-$TARGET_HOME/.config}"
DATA_ROOT="${XDG_DATA_HOME:-$TARGET_HOME/.local/share}"
STATE_ROOT="${XDG_STATE_HOME:-$TARGET_HOME/.local/state}"

HOST_CONFIG_DIR="$CONFIG_ROOT/nc-host"
HOST_DATA_DIR="$DATA_ROOT/nc-host"
HOST_STATE_DIR="$STATE_ROOT/nc-host"
HOST_GAMES_DIR=${GAMES_ROOT:-"$HOST_DATA_DIR/games"}
HOST_CONFIG_PATH="$HOST_CONFIG_DIR/host.kdl"
HOST_IDENTITY_PATH="$HOST_CONFIG_DIR/host.nsec"
GAME_DIR="$HOST_GAMES_DIR/$GAME_ID"
LOG_DIR="/var/log/nc-host"
SERVICE_DIR="/etc/sv/$SERVICE_NAME"
SERVICE_LINK="/var/service/$SERVICE_NAME"

echo "Building nc-host..."
QUOTED_BUILD_RUST_DIR=$(shell_quote "$RUST_DIR")
run_as_target sh -c "cd $QUOTED_BUILD_RUST_DIR && cargo build -p nc-host"

run_as_target mkdir -p "$HOST_CONFIG_DIR" "$HOST_GAMES_DIR" "$HOST_STATE_DIR"
run_as_target chmod 700 "$HOST_CONFIG_DIR"

if [ ! -f "$HOST_IDENTITY_PATH" ]; then
  echo "Generating host identity..."
  run_as_target "$HOST_BIN" nostr init --path "$HOST_IDENTITY_PATH"
fi
run_as_target chmod 600 "$HOST_IDENTITY_PATH"

HOST_NPUB=$(grep -m1 '^npub1' "$HOST_IDENTITY_PATH" || true)
if [ -z "$HOST_NPUB" ]; then
  echo "error: failed to read npub from $HOST_IDENTITY_PATH" >&2
  exit 1
fi

CONFIG_TMP=$(mktemp)
{
  printf '%s\n' 'host {'
  printf '    games-root "%s"\n' "$HOST_GAMES_DIR"
  printf '    relay-url "%s"\n' "$RELAY_URL"
  printf '    invite-relay-host "%s"\n' "$INVITE_RELAY_HOST"
  printf '    identity-path "%s"\n' "$HOST_IDENTITY_PATH"
  printf '    sysop-contact-npub "%s"\n' "$HOST_NPUB"
  printf '%s\n' '    sysop-contact-label "localhost dev host"'
  printf '%s\n' '}'
} > "$CONFIG_TMP"
chmod 644 "$CONFIG_TMP"
run_as_target cp "$CONFIG_TMP" "$HOST_CONFIG_PATH"
rm -f "$CONFIG_TMP"

if [ "$CREATE_GAME" -eq 1 ]; then
  if [ -f "$GAME_DIR/hosted.db" ]; then
    echo "Hosted game already exists: $GAME_DIR"
  else
    echo "Creating hosted dev game: $GAME_DIR"
    run_as_target "$HOST_BIN" new-game "$GAME_DIR" --players "$PLAYERS" --name "$GAME_NAME" --tier "$TIER"
  fi
  run_as_target "$HOST_BIN" settings set --dir "$GAME_DIR" --recruiting new_players --lobby public --summary "Localhost nc-host dev sandbox" --host-alias localhost
fi

RUN_TMP=$(mktemp)
QUOTED_TARGET_HOME=$(shell_quote "$TARGET_HOME")
QUOTED_RUST_DIR=$(shell_quote "$RUST_DIR")
QUOTED_HOST_BIN=$(shell_quote "$HOST_BIN")
QUOTED_HOST_GAMES_DIR=$(shell_quote "$HOST_GAMES_DIR")
QUOTED_HOST_CONFIG_PATH=$(shell_quote "$HOST_CONFIG_PATH")
QUOTED_HOST_IDENTITY_PATH=$(shell_quote "$HOST_IDENTITY_PATH")
QUOTED_RUN_AS_USER=$(shell_quote "$RUN_AS_USER")
{
  printf '%s\n' '#!/bin/sh'
  printf '%s\n' 'exec 2>&1'
  printf 'export HOME=%s\n' "$QUOTED_TARGET_HOME"
  printf '%s\n' 'export RUST_BACKTRACE=1'
  printf 'cd %s || exit 1\n' "$QUOTED_RUST_DIR"
  printf 'exec chpst -u %s %s --log-level info serve --root %s --config %s --identity %s\n' \
    "$QUOTED_RUN_AS_USER" "$QUOTED_HOST_BIN" "$QUOTED_HOST_GAMES_DIR" "$QUOTED_HOST_CONFIG_PATH" "$QUOTED_HOST_IDENTITY_PATH"
} > "$RUN_TMP"

LOG_TMP=$(mktemp)
QUOTED_LOG_DIR=$(shell_quote "$LOG_DIR")
{
  printf '%s\n' '#!/bin/sh'
  printf 'exec chpst -u %s svlogd -tt %s\n' "$QUOTED_RUN_AS_USER" "$QUOTED_LOG_DIR"
} > "$LOG_TMP"

run_root mkdir -p "$SERVICE_DIR/log" "$LOG_DIR"
run_root chown "$RUN_AS_USER:$RUN_AS_USER" "$LOG_DIR"
run_root install -m 0755 "$RUN_TMP" "$SERVICE_DIR/run"
run_root install -m 0755 "$LOG_TMP" "$SERVICE_DIR/log/run"
rm -f "$RUN_TMP" "$LOG_TMP"

if [ ! -e "$SERVICE_LINK" ]; then
  run_root ln -s "$SERVICE_DIR" "$SERVICE_LINK"
fi

if [ "$START_SERVICE" -eq 1 ]; then
  echo "Starting $SERVICE_NAME..."
  run_root sv restart "$SERVICE_NAME" || {
    sleep 1
    run_root sv restart "$SERVICE_NAME"
  }
else
  echo "Installed $SERVICE_NAME without starting it."
fi

echo
echo "Localhost nc-host runit service installed:"
echo "  user:     $RUN_AS_USER"
echo "  config:   $HOST_CONFIG_PATH"
echo "  identity: $HOST_IDENTITY_PATH"
echo "  games:    $HOST_GAMES_DIR"
echo "  service:  $SERVICE_DIR"
echo "  logs:     $LOG_DIR/current"
echo
echo "Check status:"
echo "  sudo sv status $SERVICE_NAME"
echo "  $HOST_BIN status --config $HOST_CONFIG_PATH"
