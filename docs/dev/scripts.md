# Dev Scripts

This note covers the local developer-facing bootstrap script used to create a
busy Rust-native campaign for TUI testing.

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

## Usage

Run from the repo root:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force
```

Then enter as player 1:

```bash
python3 scripts/run_client.py /tmp/ec-player1-ui --player 1
```

## Useful Options

### `--turn`

`--turn` controls how many Rust maintenance cycles are applied after seeding.

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
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --players 12 --year 3012 --seed 1515
```

## Recommended Flow

For a large TUI pass that starts on turn 4:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --players 12 --turn 4
python3 scripts/run_client.py /tmp/ec-player1-ui --player 1
```

## Notes

- The script is intentionally optimized for player-1 UI coverage, not balanced gameplay.
- It does not depend on `harness play-until` or a bot-conductor flow.
- It uses Rust runtime state as the source of truth.
- Player 1 gets the rich seeded backlog and foreign intel; the other empires mainly exist to make tables, rankings, diplomacy, and database screens busy.
