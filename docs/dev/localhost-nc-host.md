# Localhost `nc-host` Lab

This is the dev-only localhost hosted lab for the relay-native stack:

- `nostr-relay.service` is the relay
- `nc-host.service` is the hosted game server
- `nc-helm` is the hosted client

This is not the current public release path. Public play is still centered on
`nc-game`, `nc-door`, and `nc-sysop`.

## Local Paths

The localhost lab uses user-local paths:

- config: `~/.config/nc-host/host.kdl`
- identity: `~/.config/nc-host/host.nsec`
- service: `~/.config/systemd/user/nc-host.service`
- games root: `~/.local/share/nc-host/games`

The host config uses:

- `relay-url "ws://127.0.0.1:8080"`
- `invite-relay-host "localhost:8080"`

The relay itself should advertise:

- `relay_url = "ws://localhost:8080/"`

## Install Or Refresh

Run from the repo root:

```bash
./scripts/install_nc_host_user_service.sh
```

This script:

- builds `target/debug/nc-host`
- creates the user-local `nc-host` config/data directories
- generates `host.nsec` if missing
- writes `host.kdl`
- installs `nc-host.service`
- removes stale `ec-gate.service` and `ec4x-daemon.service`
- reloads the user systemd manager
- restarts `nostr-relay.service`
- enables and starts `nc-host.service`

Useful options:

```bash
./scripts/install_nc_host_user_service.sh --games-root /tmp/nc-host-games --no-restart
./scripts/install_nc_host_user_service.sh --relay-url ws://127.0.0.1:8080 --invite-relay-host localhost:8080
```

## Daily Dev Flow

After `nc-host` code changes:

```bash
cd rust
cargo build -p nc-host
systemctl --user restart nc-host.service
```

Run the hosted client against the local relay:

```bash
cd rust
cargo run -q -p nc-helm -- --relay ws://localhost:8080
```

## Validation

Check services:

```bash
systemctl --user status nostr-relay.service
systemctl --user status nc-host.service
```

Check host status:

```bash
cd rust
cargo run -q -p nc-host -- status --config ~/.config/nc-host/host.kdl --root ~/.local/share/nc-host/games
```

The healthy localhost baseline is:

- relay connected
- host service active
- zero config parse errors
- zero games until you create hosted game directories

## Notes

- Screen lock is fine for this lab.
- Auto-suspend is the real risk; if the machine sleeps, both user services stop until wake.
- No `linger` setup is required unless you want services to survive full logout.
