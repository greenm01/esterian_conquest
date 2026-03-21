# Scripts

This directory holds repeatable local test-game setup helpers for the
SQLite-native Rust workflow.

These scripts are intended for:

- creating fresh joinable test games quickly
- creating richer stress-test campaigns for UI work
- creating classic-probe campaigns that open in original `ECGAME` with a busy
  player-1 report backlog
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

### `setup_classic_probe_game.py`

Creates a fresh four-player Rust-backed campaign aimed at classic `ECGAME`
playback and display checking.

It currently:

- creates a fresh `sysop new-game` directory
- names all four players and empires
- gives player 1 multiple owned worlds and a busier fleet roster
- seeds nearby hostile target worlds and incoming enemy fleets
- runs several Rust maint turns so `RESULTS.DAT` contains a busy maint-report
  backlog when player 1 logs in
- clears routed `MESSAGES.DAT` output back out of the classic probe directory,
  because original `ECGAME` can display the maint reports from `RESULTS.DAT`
  but the Rust-only routed `MESSAGES.DAT` format is not classic-compatible
- prepares the classic login alias for player 1
- launches original `ECGAME` through DOSBox-X unless `--no-launch` is used

Example:

```bash
python3 scripts/setup_classic_probe_game.py /tmp/ec-classic-probe --force
```

Dry-run example:

```bash
python3 scripts/setup_classic_probe_game.py /tmp/ec-classic-probe --force --no-launch
```

Use this when you want:

- the original DOS client, not `ec-client`
- a busy unread-report backlog for player 1
- multiple fleets and planets to inspect in classic menus
- a practical hybrid-loop smoke test after Rust maint changes

### `run_classic_report_probe.sh`

Thin wrapper around `setup_classic_probe_game.py` for the most common
"open classic ECGAME with pending reports" workflow.

If you do not pass a target directory, it uses:

```bash
/tmp/ec-classic-report-probe
```

Example:

```bash
./scripts/run_classic_report_probe.sh
```

Custom target example:

```bash
./scripts/run_classic_report_probe.sh /tmp/ec-report-format-probe
```

Pass-through example:

```bash
./scripts/run_classic_report_probe.sh --turns 6 --alias SYSOP
```

This wrapper always adds `--force`, so rerunning it refreshes the probe
directory before launch.

### `build_release_packages.py`

Builds the reproducible demo-ready release zips under `releases/`.

It currently:

- builds one archive with the original packed binaries
- builds one archive with the curated runnable unlocked binaries
- copies the original `.DOC` manuals into each package
- seeds both packages with the preserved `fixtures/ecutil-init/v1.5` game
  directory
- generates the known-good local-console `CHAIN.TXT`
- refreshes `EC_UNLOCKED/` first when the unlocked variant is selected
- validates the generated archives when `--verify` is passed

Example:

```bash
python3 scripts/build_release_packages.py --verify
```

Classic-only example:

```bash
python3 scripts/build_release_packages.py --variant classic --verify
```

Use this when you want:

- a reproducible bundle to hand to emulator developers
- a small local package that opens with the repo's known-good `CHAIN.TXT`
- a clean split between original packed and curated runnable unlocked variants

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

### Classic ECGAME playback test

```bash
python3 scripts/setup_classic_probe_game.py /tmp/ec-classic-probe --force
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
