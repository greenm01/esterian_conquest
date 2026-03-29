# Scripts

This directory holds repeatable local test-game setup helpers for the
SQLite-native Rust workflow.

These scripts are intended for:

- creating fresh joinable test games quickly
- creating richer stress-test campaigns for UI work
- creating classic-probe campaigns that open in original `ECGAME` with a busy
  player-1 report backlog
- bootstrapping a VPS host for DB-only Rust campaigns
- launching `ec-game` against a chosen campaign and player seat
- building standalone release bundles for playtesting

The current boundary is:

- `ec-game` runs from `ecgame.db`
- `maint-rust` runs from `ecgame.db`
- `ec-cli` is the setup and classic `.DAT` bridge/tooling surface

So these scripts call `ec-cli` to build or mutate campaigns, then launch
`ec-game` directly.

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

### `install_vps.sh`

Root-only idempotent bootstrap for the recommended VPS layout.

It:

- creates the dedicated `ecgame` service user
- ensures the service user has a real shell for forced SSH commands
- creates `/etc/ec-gate`, `/var/lib/ec-gate/keys`, and `/srv/ec/games`
- installs `ec-game` and `ec-sysop` into `/usr/local/bin`
- installs `/usr/local/bin/ec-gate-keys`
- writes `/etc/ec-gate/config.kdl`
- installs the `ec-nostr.service`, `ec-maint-all.service`, and `ec-maint-all.timer` units
- installs an `sshd` drop-in for the service user
- initializes `/etc/ec-gate/identity.kdl` if missing

Example:

```bash
sudo ./scripts/install_vps.sh \
  --relay wss://relay.example.com \
  --ssh-host play.example.com
```

This script never creates classic `.DAT` files or copies DOS artifacts into
per-game directories. Hosted Rust campaigns remain DB-only.

Host game-registry edits still happen as `root` because `/etc/ec-gate/config.kdl`
is host-owned. After `host games add/remove`, restart `ec-nostr.service` so the
daemon reloads the updated game list.

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

### `setup_player1_tui_stress_game.py`

Creates a larger twelve-player campaign aimed specifically at **player 1 TUI
coverage** without running a bot-played campaign.

It currently:

- creates a fresh engine-backed `sysop new-game` campaign
- uses a fixed default map seed for reproducible placement
- supports `--year`, `--players`, `--seed`, and `--turn`
- names all twelve empires and assigns them varied tax rates
- gives player 1 a large owned-colony footprint plus a much larger fleet roster
- stages empty and loaded troop transports at Aurora Prime for load/unload testing
- seeds player 1 with active starbases, rich unread report blocks, and queued mail
- injects mixed foreign-world intel for player 1 so database/detail screens show
  unknown, partial, and full scout-quality rows

Example:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force
```

Turn-4 example:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --turn 4
```

Explicit-seed example:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --players 12 --seed 1515
```

Explicit-year example:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --players 12 --year 3012 --seed 1515
```

Use this when you want:

- a busy player 1 startup flow with unread reports and messages
- fleet, planet, database, and rankings tables that scroll immediately
- `INFO ABOUT A PLANET` to show both owned detail and varied foreign intel
- starbase, transport, build, and stardock screens populated on first launch

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

- the original DOS client, not `ec-game`
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

Builds the reproducible EC v1.5 DOS release zips under `releases/`.

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

### `build_playtest_bundle.py`

Builds a standalone Linux or macOS `tar.gz` playtest bundle under `releases/`.

It currently:

- builds either:
  - the internal combined private-beta bundle
  - or a public `ec-connect` player archive
- includes the matching public PDF manuals under `docs/`
- writes `README.md` and `BUILD-INFO.txt` into the bundle root
- can unpack and smoke-test the bundle when `--verify` is passed
- defaults to the current host Rust target, with explicit support for:
  - `x86_64-unknown-linux-gnu`
  - `aarch64-apple-darwin`
  - `x86_64-apple-darwin`

Example:

```bash
python3 scripts/build_playtest_bundle.py --verify
```

Public player archive example:

```bash
python3 scripts/build_playtest_bundle.py --artifact ec-connect --verify
```

Explicit Apple Silicon example:

```bash
python3 scripts/build_playtest_bundle.py --artifact ec-connect --target aarch64-apple-darwin --verify
```

Use this when you want:

- a native Linux or macOS archive without requiring a Rust toolchain
- either a public `ec-connect` player package or the internal combined bundle
- a quick way to hand players or testers the right manual with the binary

The combined bundle is an internal/private-beta helper. The `ec-connect`
artifact is the public player-facing archive.

The public `ec-connect` release flow currently publishes:

- `x86_64-unknown-linux-gnu`
- `aarch64-apple-darwin`

### `build_linux_playtest_bundle.py`

Compatibility wrapper around `build_playtest_bundle.py` that keeps the old
Linux x64 command working:

```bash
python3 scripts/build_linux_playtest_bundle.py --verify
```

### `publish_release_packages.sh`

Builds the selected DOS release bundles and/or `ec-connect` player archives,
verifies them, then uploads the generated assets to an existing GitHub Release
with `gh release upload --clobber`.

Default example:

```bash
./scripts/publish_release_packages.sh
```

Custom tag example:

```bash
./scripts/publish_release_packages.sh --tag release-artifacts
```

Unlocked-only example:

```bash
./scripts/publish_release_packages.sh --variant unlocked
```

Linux player archive example:

```bash
./scripts/publish_release_packages.sh --ec-connect-target x86_64-unknown-linux-gnu
```

Use this when you want:

- the easiest release workflow for DOS bundles and public `ec-connect` archives
- the generated release assets to stay untracked locally under `releases/`
- the public downloadable copies to live on GitHub Releases instead of `main`

### `run_client.py`

Launches `ec-game` against a chosen campaign directory and player seat.

Example:

```bash
python3 scripts/run_client.py /tmp/ec-ui-stress --player 1
```

Release build example:

```bash
python3 scripts/run_client.py /tmp/ec-ui-stress --player 1 --release
```

By default `run_client.py` launches `ec-game` against the existing
`ecgame.db` runtime state and does not refresh from classic `.DAT` files.
This preserves joins, theme choices, and other in-client changes across
re-entry.

If you intentionally changed the `.DAT` files outside the Rust runtime and
need to resync the runtime DB before launch, opt in explicitly:

```bash
python3 scripts/run_client.py /tmp/ec-ui-stress --player 1 --refresh-from-dat
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

### Player 1 TUI torture test

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force
python3 scripts/run_client.py /tmp/ec-player1-ui --player 1
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
