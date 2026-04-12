# Dev Scripts

This note covers the local developer-facing bootstrap scripts used for both the
Rust-native TUI test flow and the dev-only localhost hosted lab.

## Localhost Hosted Lab

Use [scripts/install_nc_host_user_service.sh](../../scripts/install_nc_host_user_service.sh)
when you want a stable localhost `nc-host` + `nc-dash` user-service setup.

It installs and refreshes:

- `~/.config/nc-host/host.kdl`
- `~/.config/nc-host/host.nsec`
- `~/.config/systemd/user/nc-host.service`
- `~/.local/share/nc-host/games`

Run from the repo root:

```bash
./scripts/install_nc_host_user_service.sh
```

This script is dev-only. It is not the public production deployment path.

Useful validation commands:

```bash
systemctl --user status nostr-relay.service
systemctl --user status nc-host.service
cd rust
cargo run -q -p nc-host -- status --config ~/.config/nc-host/host.kdl --root ~/.local/share/nc-host/games
```

## Player 1 TUI Stress Game

Use [scripts/setup_player1_tui_stress_game.py](../../scripts/setup_player1_tui_stress_game.py)
when you want a campaign that drops player 1 into a loaded test environment
without running a bot-played history.

It is designed to populate the client with enough live state to exercise:

- startup unread reports and messages
- main/general/planet/fleet/starbase menu flows
- long fleet and planet tables
- total planet database scrolling
- owned `INFO ABOUT A PLANET` detail
- foreign intel `INFO ABOUT A PLANET` detail
- build, commission, transport, starbase, and message screens
- partial starmap and rankings with non-trivial data

The script creates a Rust-backed campaign and seeds:

- an engine-generated `sysop new-game` map, not a fixed coordinate template

- player 1 with many owned worlds
- player 1 with a large mixed fleet roster
- empty troop transports at homeworld for load-armies testing
- loaded troop transports for unload testing
- active player-1 starbases
- player-1-only queued mail and report backlog
- mixed foreign intel for player 1, including partial and full scout-style data
- optional localhost hosted returning-player fixture data for `nc-connect`

## Usage

Run from the repo root:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force
```

Then enter as player 1:

```bash
python3 scripts/run_client.py /tmp/ec-player1-ui --player 1
```

`run_client.py` now preserves the existing `ncgame.db` runtime state by
default. This is the normal workflow for repeated `nc-game` login/logout
testing.

If you intentionally changed the classic `.DAT` files outside the Rust runtime
and want to refresh `ncgame.db` from them before launch, opt in explicitly:

```bash
python3 scripts/run_client.py /tmp/ec-player1-ui --player 1 --refresh-from-dat
```

## Useful Options

### `--turn`

`--turn` controls how many Rust maintenance cycles are applied after seeding.
The stress fixture always starts from year `3000`, so use `--turn` to advance
into later years.

- `--turn 1`: seeded baseline, no maintenance run
- `--turn 4`: seeded baseline plus three `maint-rust` passes

Example:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --turn 4
```

### `--players`

`--players` changes the player count for the stress template.

Supported range:

- `4..12`

The script validates:

- player count stays within the supported template range
- seeded homeworld coordinates fit inside the expected map-size tier

Map-size tiers follow the Rust runtime rules:

- `1..4` players -> `18x18`
- `5..9` players -> `27x27`
- `10..16` players -> `36x36`
- `17..25` players -> `45x45`

This script currently supports only the first three ranges up to 12 players,
because the seeded empire/homeworld template is defined for 12 seats.

Example:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --players 9 --turn 3
```

### `--seed`

`--seed` controls the engine-backed map generation used for the base campaign.

The default is fixed so repeated UI test runs land on the same generated map.

Example:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --players 12 --seed 1515
```

## Recommended Flow

For a large TUI pass that starts on turn 4:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --players 12 --turn 4
python3 scripts/run_client.py /tmp/ec-player1-ui --player 1
```

## Local Hosted GUI Invite Test

If you want to test the real `nc-connect` GUI invite flow locally against the
stress game, keep the same `/tmp/ec-player1-ui` campaign but start a local gate
publisher for it:

```bash
./scripts/start_local_gui_hosted_test.sh --dir /tmp/ec-player1-ui
```

This helper:

- checks that `/tmp/ec-player1-ui/ncgame.db` exists
- prints pending invite codes when the game still has unclaimed seats
- prints claimed-seat identities when the game has already been seeded for returning-player reconnect tests
- requires a relay already listening at `ws://localhost:8080`
- writes a temporary gate config and identity under `/tmp/ec-local-gate`
- defaults loopback localhost to the current user plus `~/.ssh/authorized_keys`
- still supports explicit `--ssh-user` / `--auth-keys-*` overrides
- starts `nc-sysop nostr serve` for that game
- prints raw invite codes such as `victim-sickness@localhost:8080`

Then, in another terminal, run the GUI and paste one of the printed invite
codes:

```bash
cd rust
cargo run -q -p nc-connect --bin nc-connect
```

Notes:

- This is the real hosted Nostr discovery path, not a direct local-game shortcut.
- The relay is still an external prerequisite; the helper does not install or
  launch a relay binary for you.
- For same-machine hosted play, the normal path is to run this helper without
  `sudo`; it publishes your current login plus `~/.ssh/authorized_keys` unless
  you override it.
- This is distinct from direct localhost `nc-game` play. Through `nc-connect`,
  localhost-hosted sessions still use the normal hosted SSH transport.

## Localhost Returning-Player Fixture

If you want `nc-connect` to show a localhost stress game in the picker
immediately, seed a claimed hosted seat plus an isolated `nc-connect` state
root:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --hosted-claim-player 1 --hosted-nsec-file /path/to/player.nsec
```

That flow:

- claims one hosted seat for the supplied identity
- seeds an isolated keychain/cache/config tree under `/tmp`
- prints the exact `XDG_CONFIG_HOME=... XDG_DATA_HOME=... cargo run ...` command
- is meant for returning-player reconnect testing, not first-join testing

## Notes

- The script is intentionally optimized for player-1 UI coverage, not balanced gameplay.
- It does not depend on `harness play-until` or a bot-conductor flow.
- It uses Rust runtime state as the source of truth.
- Player 1 gets the rich seeded backlog and foreign intel; the other empires mainly exist to make tables, rankings, diplomacy, and database screens busy.

## nc-dash Map Lab

Use [scripts/setup_nc_dash_lab.py](../../scripts/setup_nc_dash_lab.py) when you
want a quick `nc-dash` pass across all four map-size tiers instead of a
single-player stress fixture.

It wraps `nc-cli harness seed-nc-dash-lab` and creates:

- `map18-p4`
- `map27-p9`
- `map36-p16`
- `map45-p25`

Each seeded campaign is ready to open directly in `nc-dash`, and the script
writes a `README.txt` manifest under the chosen lab root.

## Usage

Run from the repo root:

```bash
python3 scripts/setup_nc_dash_lab.py --root /tmp/nc-dash-lab --force
```

Then launch whichever map tier you want to inspect:

```bash
cd rust
cargo run -q -p nc-dash -- /tmp/nc-dash-lab/map45-p25
```

To seed and jump directly into one tier in one step:

```bash
python3 scripts/setup_nc_dash_lab.py --root /tmp/nc-dash-lab --force --launch map36-p16
```
