#!/usr/bin/env bash
set -euo pipefail

EC_USER="ecgame"
GAMES_ROOT="/srv/ec/games"
CONFIG_DIR="/etc/ec-gate"
STATE_DIR="/var/lib/ec-gate"
AUTH_KEYS_METHOD="command"
AUTH_KEYS_PATH="/var/lib/ec-gate/keys"
EC_GAME_SRC="${PWD}/rust/target/release/ec-game"
EC_SYSOP_SRC="${PWD}/rust/target/release/ec-sysop"
EC_GAME_DEST="/usr/local/bin/ec-game"
EC_SYSOP_DEST="/usr/local/bin/ec-sysop"
EC_GATE_KEYS_DEST="/usr/local/bin/ec-gate-keys"
SSH_PORT="22"
KEY_TTL="60"
ENABLE_SERVICES="1"
OVERWRITE_CONFIG="0"
RELAY_URL=""
SSH_HOST=""
declare -a GAMES=()

usage() {
  cat <<'EOF'
Usage:
  sudo ./scripts/install_vps.sh --relay <wss://relay> --ssh-host <host> [options]

Options:
  --ec-user <name>                 Service user. Default: ecgame
  --games-root <path>              Parent directory for campaign dirs. Default: /srv/ec/games
  --relay <url>                    Nostr relay URL written to /etc/ec-gate/config.kdl
  --ssh-host <host>                Public SSH host sent to players
  --ssh-port <port>                Public SSH port. Default: 22
  --auth-keys-method <command|file>
                                   Authorized keys backend. Default: command
  --auth-keys-path <path>          Authorized keys dir/file. Default: /var/lib/ec-gate/keys
  --key-ttl <seconds>              Ephemeral SSH key TTL. Default: 60
  --ec-game-src <path>             Source binary to install. Default: ./rust/target/release/ec-game
  --ec-sysop-src <path>            Source binary to install. Default: ./rust/target/release/ec-sysop
  --game <dir>                     Register a game directory in /etc/ec-gate/config.kdl (repeatable)
  --overwrite-config               Rewrite /etc/ec-gate/config.kdl from the supplied flags
  --skip-enable                    Install files but do not enable/restart systemd units
  --help                           Show this help
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --ec-user)
      EC_USER="$2"
      shift 2
      ;;
    --games-root)
      GAMES_ROOT="$2"
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
    --auth-keys-method)
      AUTH_KEYS_METHOD="$2"
      shift 2
      ;;
    --auth-keys-path)
      AUTH_KEYS_PATH="$2"
      shift 2
      ;;
    --key-ttl)
      KEY_TTL="$2"
      shift 2
      ;;
    --ec-game-src)
      EC_GAME_SRC="$2"
      shift 2
      ;;
    --ec-sysop-src)
      EC_SYSOP_SRC="$2"
      shift 2
      ;;
    --game)
      GAMES+=("$2")
      shift 2
      ;;
    --overwrite-config)
      OVERWRITE_CONFIG="1"
      shift
      ;;
    --skip-enable)
      ENABLE_SERVICES="0"
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

if [ "$(id -u)" -ne 0 ]; then
  echo "error: run this installer as root" >&2
  exit 1
fi

if [ -z "$RELAY_URL" ] || [ -z "$SSH_HOST" ]; then
  echo "error: --relay and --ssh-host are required" >&2
  usage >&2
  exit 1
fi

case "$AUTH_KEYS_METHOD" in
  command|file) ;;
  *)
    echo "error: --auth-keys-method must be command or file" >&2
    exit 1
    ;;
esac

if [ ! -x "$EC_GAME_SRC" ]; then
  echo "error: missing executable ec-game binary at $EC_GAME_SRC" >&2
  exit 1
fi

if [ ! -x "$EC_SYSOP_SRC" ]; then
  echo "error: missing executable ec-sysop binary at $EC_SYSOP_SRC" >&2
  exit 1
fi

if command -v nologin >/dev/null 2>&1; then
  NOLOGIN_SHELL="$(command -v nologin)"
elif [ -x /usr/sbin/nologin ]; then
  NOLOGIN_SHELL="/usr/sbin/nologin"
else
  NOLOGIN_SHELL="/bin/false"
fi

if ! id "$EC_USER" >/dev/null 2>&1; then
  useradd --system --home "$STATE_DIR" --shell "$NOLOGIN_SHELL" --create-home "$EC_USER"
fi

install -d -m 0750 -o "$EC_USER" -g "$EC_USER" "$GAMES_ROOT"
install -d -m 0750 -o "$EC_USER" -g "$EC_USER" "$STATE_DIR"
if [ "$AUTH_KEYS_METHOD" = "command" ]; then
  install -d -m 0700 -o "$EC_USER" -g "$EC_USER" "$AUTH_KEYS_PATH"
elif [ ! -e "$AUTH_KEYS_PATH" ]; then
  install -m 0600 -o "$EC_USER" -g "$EC_USER" /dev/null "$AUTH_KEYS_PATH"
fi
install -d -m 0750 -o root -g "$EC_USER" "$CONFIG_DIR"

for game_dir in "${GAMES[@]}"; do
  install -d -m 0750 -o "$EC_USER" -g "$EC_USER" "$game_dir"
done

install -m 0755 "$EC_GAME_SRC" "$EC_GAME_DEST"
install -m 0755 "$EC_SYSOP_SRC" "$EC_SYSOP_DEST"

cat >"$EC_GATE_KEYS_DEST" <<EOF
#!/usr/bin/env sh
set -eu
EXPECTED_USER="$EC_USER"
KEY_DIR="$AUTH_KEYS_PATH"

if [ "\${1:-}" != "\$EXPECTED_USER" ]; then
  exit 0
fi

if [ ! -d "\$KEY_DIR" ]; then
  exit 0
fi

now=\$(date +%s)
for key_file in "\$KEY_DIR"/*.key; do
  [ -e "\$key_file" ] || continue
  expires_line=\$(sed -n '1p' "\$key_file" 2>/dev/null || true)
  case "\$expires_line" in
    expires=*)
      expires_at=\${expires_line#expires=}
      ;;
    *)
      continue
      ;;
  esac
  case "\$expires_at" in
    ''|*[!0-9]*)
      continue
      ;;
  esac
  if [ "\$expires_at" -gt "\$now" ]; then
    sed -n '2p' "\$key_file" 2>/dev/null || true
  fi
done
EOF
chmod 0755 "$EC_GATE_KEYS_DEST"

CONFIG_PATH="$CONFIG_DIR/config.kdl"
if [ ! -f "$CONFIG_PATH" ] || [ "$OVERWRITE_CONFIG" = "1" ]; then
  {
    printf 'relay "%s"\n' "$RELAY_URL"
    printf 'ssh-host "%s"\n' "$SSH_HOST"
    printf 'ssh-port %s\n' "$SSH_PORT"
    printf 'ssh-user "%s"\n' "$EC_USER"
    printf 'ec-game-path "%s"\n' "$EC_GAME_DEST"
    printf 'auth-keys-method "%s"\n' "$AUTH_KEYS_METHOD"
    printf 'auth-keys-path "%s"\n' "$AUTH_KEYS_PATH"
    printf 'key-ttl %s\n' "$KEY_TTL"
    for game_dir in "${GAMES[@]}"; do
      printf 'game "%s"\n' "$game_dir"
    done
  } >"$CONFIG_PATH"
  chown root:"$EC_USER" "$CONFIG_PATH"
  chmod 0640 "$CONFIG_PATH"
fi

cat >/etc/systemd/system/ec-nostr.service <<EOF
[Unit]
Description=Esterian Conquest Nostr Session Daemon
After=network-online.target sshd.service
Wants=network-online.target

[Service]
Type=simple
User=$EC_USER
Group=$EC_USER
ExecStart=$EC_SYSOP_DEST nostr serve --config $CONFIG_PATH --identity $CONFIG_DIR/identity.kdl
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

cat >/etc/systemd/system/ec-maint-all.service <<EOF
[Unit]
Description=Esterian Conquest maintenance sweep
After=network-online.target

[Service]
Type=oneshot
User=$EC_USER
Group=$EC_USER
ExecStart=$EC_SYSOP_DEST maint-all --config $CONFIG_PATH
EOF

cat >/etc/systemd/system/ec-maint-all.timer <<'EOF'
[Unit]
Description=Run Esterian Conquest maintenance sweep every five minutes

[Timer]
OnCalendar=*:0/5
Persistent=true
Unit=ec-maint-all.service

[Install]
WantedBy=timers.target
EOF

install -d -m 0755 /etc/ssh/sshd_config.d
cat >/etc/ssh/sshd_config.d/ecgame.conf <<EOF
Match User $EC_USER
    AuthorizedKeysCommand $EC_GATE_KEYS_DEST %u
    AuthorizedKeysCommandUser $EC_USER
    PasswordAuthentication no
    PubkeyAuthentication yes
    PermitTTY yes
    X11Forwarding no
    AllowTcpForwarding no
    AllowAgentForwarding no
    PermitOpen none
EOF

if [ ! -f "$CONFIG_DIR/identity.kdl" ]; then
  "$EC_SYSOP_DEST" nostr init --identity "$CONFIG_DIR/identity.kdl"
fi
chown root:"$EC_USER" "$CONFIG_DIR/identity.kdl"
chmod 0640 "$CONFIG_DIR/identity.kdl"

systemctl daemon-reload
if systemctl list-unit-files sshd.service >/dev/null 2>&1; then
  systemctl reload sshd || true
elif systemctl list-unit-files ssh.service >/dev/null 2>&1; then
  systemctl reload ssh || true
fi

if [ "$ENABLE_SERVICES" = "1" ]; then
  systemctl enable --now ec-maint-all.timer
  systemctl enable --now ec-nostr.service
fi

cat <<EOF
Installed Esterian Conquest VPS layout:
  service user: $EC_USER
  config dir:   $CONFIG_DIR
  state dir:    $STATE_DIR
  games root:   $GAMES_ROOT
  ec-game:      $EC_GAME_DEST
  ec-sysop:     $EC_SYSOP_DEST
  gate config:  $CONFIG_PATH

Next steps:
  1. Create a game:
     sudo -u $EC_USER $EC_SYSOP_DEST new-game $GAMES_ROOT/<slug> --name "<Game Name>" --players 4
  2. Register it:
     sudo -u $EC_USER $EC_SYSOP_DEST host games add --config $CONFIG_PATH --dir $GAMES_ROOT/<slug>
  3. Inspect hosted seats:
     sudo -u $EC_USER $EC_SYSOP_DEST nostr seats --dir $GAMES_ROOT/<slug>
EOF
