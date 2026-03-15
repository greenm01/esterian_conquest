# Scripts

This directory holds repeatable local test-game setup helpers for the
SQLite-native Rust workflow.

These scripts are intended for:

- creating fresh joinable test games quickly
- creating richer stress-test campaigns for UI work
- launching `ec-client` against a chosen campaign and player seat

The current boundary is:

- `ec-client` runs from `ecgame.db`
- `maint-rust` runs from `ecgame.db`
- `ec-cli` is the setup and classic `.DAT` bridge/tooling surface

So these scripts call `ec-cli` to build or mutate campaigns, then launch the
Rust client directly.

## Prerequisites

Run all examples from the repo root:

```bash
cd /home/mag/dev/esterian_conquest
```

The scripts expect:

- a working Rust toolchain
- `cargo` available in `PATH`
- the repo workspace intact under `rust/`

## Available Scripts

### `new_test_game.py`

Creates a fresh joinable game directory using the normal Rust sysop flow.

Example:

```bash
python3 scripts/new_test_game.py /tmp/ec-join-test --players 9 --force
```

Notes:

- `--players` is required
- `--seed` is optional
- `--force` removes the target directory first

This is the right default when you want a clean first-time/join flow for UI
testing.

### `setup_ui_stress_game.py`

Creates a richer four-player campaign specifically for testing the client and
maintenance behavior.

It currently:

- builds a year-`3010` game
- names all four players and empires
- gives each empire multiple owned planets
- gives player 1 a much larger and more varied fleet roster
- assigns a spread of fleet orders so menus have more interesting data

Example:

```bash
python3 scripts/setup_ui_stress_game.py /tmp/ec-ui-stress --force
```

Use this when you want:

- busy fleet lists
- planet-management screens with more than one colony
- more meaningful general/intel screens
- a useful maintenance test bed

### `run_client.py`

Launches the Rust client against a chosen campaign directory and player seat.

Example:

```bash
python3 scripts/run_client.py /tmp/ec-ui-stress --player 1
```

Release build example:

```bash
python3 scripts/run_client.py /tmp/ec-ui-stress --player 1 --release
```

## Recommended Workflow

### Clean join/onboarding test

```bash
python3 scripts/new_test_game.py /tmp/ec-join-test --players 4 --force
python3 scripts/run_client.py /tmp/ec-join-test --player 1
```

### Rich UI / command-menu test

```bash
python3 scripts/setup_ui_stress_game.py /tmp/ec-ui-stress --force
python3 scripts/run_client.py /tmp/ec-ui-stress --player 1
```

### Maintenance regression pass on a scripted game

```bash
cd rust
cargo run -q -p ec-cli -- maint-rust /tmp/ec-ui-stress 1
```

### Export a scripted game back to classic `.DAT`

```bash
cd rust
cargo run -q -p ec-cli -- db-export /tmp/ec-ui-stress /tmp/ec-ui-stress-exported
```

Then run the original oracle if needed:

```bash
python3 tools/ecmaint_oracle.py run /tmp/ec-ui-stress-exported
```

## Extending These Scripts

If you add new scripted setups, keep them aligned with the current project
boundary:

- prefer calling `ec-cli` commands instead of mutating SQLite directly in
  Python
- treat `ec-cli` as the stable setup surface
- keep the client/runtime SQLite-native
- use `db-export` only when you specifically need classic compatibility output

If a setup needs a new reusable state mutation, add a focused `ec-cli` command
first and keep the Python layer thin.
