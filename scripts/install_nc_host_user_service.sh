#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_DIR="$REPO_ROOT/rust"
HOST_BIN="$RUST_DIR/target/debug/nc-host"

CONFIG_ROOT="${XDG_CONFIG_HOME:-$HOME/.config}"
DATA_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}"

HOST_CONFIG_DIR="$CONFIG_ROOT/nc-host"
HOST_DATA_DIR="$DATA_ROOT/nc-host"
SYSTEMD_USER_DIR="$CONFIG_ROOT/systemd/user"
HOST_GAMES_DIR="$HOST_DATA_DIR/games"
HOST_CONFIG_PATH="$HOST_CONFIG_DIR/host.kdl"
HOST_IDENTITY_PATH="$HOST_CONFIG_DIR/host.nsec"
HOST_UNIT_PATH="$SYSTEMD_USER_DIR/nc-host.service"
RELAY_CONFIG_PATH="$CONFIG_ROOT/nostr-rs-relay/config.toml"

RELAY_URL="ws://localhost:8080"
INVITE_RELAY_HOST="localhost:8080"
RESTART_SERVICES=1

usage() {
  cat <<'EOF'
Usage:
  ./scripts/install_nc_host_user_service.sh [options]

Options:
  --games-root <path>          Override hosted games root.
  --relay-url <url>            Host relay URL. Default: ws://localhost:8080
  --invite-relay-host <host>   Invite relay host[:port]. Default: localhost:8080
  --no-restart                 Install files without restarting user services.
  --help                       Show this help.

This is a dev-only localhost installer for the nc-host + nostr-relay user-service lab.
It removes stale ec-gate/ec4x-daemon units, builds nc-host, writes ~/.config/nc-host,
and installs ~/.config/systemd/user/nc-host.service.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --games-root)
      HOST_GAMES_DIR="$2"
      shift 2
      ;;
    --relay-url)
      RELAY_URL="$2"
      shift 2
      ;;
    --invite-relay-host)
      INVITE_RELAY_HOST="$2"
      shift 2
      ;;
    --no-restart)
      RESTART_SERVICES=0
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

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command: $1" >&2
    exit 1
  fi
}

require_cmd cargo
require_cmd systemctl
require_cmd perl

mkdir -p "$HOST_CONFIG_DIR" "$HOST_GAMES_DIR" "$SYSTEMD_USER_DIR"

echo "Building nc-host..."
(cd "$RUST_DIR" && cargo build -p nc-host)

if [[ ! -f "$HOST_IDENTITY_PATH" ]]; then
  echo "Generating host identity..."
  "$HOST_BIN" nostr init --path "$HOST_IDENTITY_PATH"
fi

HOST_NPUB="$(grep -m1 '^npub1' "$HOST_IDENTITY_PATH" || true)"
if [[ -z "$HOST_NPUB" ]]; then
  echo "error: failed to read npub from $HOST_IDENTITY_PATH" >&2
  exit 1
fi

cat > "$HOST_CONFIG_PATH" <<EOF
host {
    games-root "$HOST_GAMES_DIR"
    relay-url "$RELAY_URL"
    invite-relay-host "$INVITE_RELAY_HOST"
    identity-path "$HOST_IDENTITY_PATH"
    sysop-contact-npub "$HOST_NPUB"
}
EOF

cat > "$HOST_UNIT_PATH" <<EOF
[Unit]
Description=NC Host - Hosted Nostr Game Server
After=network-online.target nostr-relay.service
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=$RUST_DIR
ExecStart=$HOST_BIN serve --root $HOST_GAMES_DIR --config $HOST_CONFIG_PATH --identity $HOST_IDENTITY_PATH
Restart=on-failure
RestartSec=3
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
EOF

if [[ -f "$RELAY_CONFIG_PATH" ]]; then
  perl -0pi -e 's#relay_url = "wss://nostr\.example\.com/"#relay_url = "ws://localhost:8080/"#g' "$RELAY_CONFIG_PATH"
fi

systemctl --user disable --now ec-gate.service ec4x-daemon.service >/dev/null 2>&1 || true
rm -f \
  "$SYSTEMD_USER_DIR/ec-gate.service" \
  "$SYSTEMD_USER_DIR/ec4x-daemon.service" \
  "$SYSTEMD_USER_DIR/default.target.wants/ec-gate.service" \
  "$SYSTEMD_USER_DIR/default.target.wants/ec4x-daemon.service"

systemctl --user daemon-reload

if [[ "$RESTART_SERVICES" -eq 1 ]]; then
  echo "Restarting localhost relay and nc-host user services..."
  systemctl --user restart nostr-relay.service
  systemctl --user enable --now nc-host.service
else
  echo "Installed nc-host files without restarting services."
  echo "Run:"
  echo "  systemctl --user daemon-reload"
  echo "  systemctl --user restart nostr-relay.service"
  echo "  systemctl --user enable --now nc-host.service"
fi

echo
echo "Localhost nc-host lab installed:"
echo "  config:   $HOST_CONFIG_PATH"
echo "  identity: $HOST_IDENTITY_PATH"
echo "  games:    $HOST_GAMES_DIR"
echo "  unit:     $HOST_UNIT_PATH"
