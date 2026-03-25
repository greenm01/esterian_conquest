# esterian_conquest

![Esterian Conquest banner](docs/assets/ec_v1.5_banner.png)

_Inspired by Esterian Conquest (c) 1992 Bentley C. Griffith.
A fan-built resurrection -- not affiliated with the original._

## Premise

Beyond the mapped frontiers of the old Esterian dominion lies a small galaxy
of contested solar systems. The old masters are gone. Their stations are
silent, their patrols vanished, and their subjects left with fleets,
factories, and enough knowledge to build empires.

EC treats that classic premise seriously. The goal is not to sand away the
identity of the original game, but to keep its campaign feel, menus, reports,
and old-school tension while replacing the DOS runtime with a modern Rust
implementation.

## Play

There are three ways to run an Esterian Conquest campaign:

- Hosted on a BBS as a door game
- Shared on a remote host over SSH
- Solo or hotseat on localhost

The most natural way to play is hosted on a BBS, the way the game was
originally designed. A sysop runs the engine, and players call in to submit
orders and read reports through the door. That async rhythm — log in, review
your empire, issue orders, log out, wait for the turn to resolve — is the
heartbeat of EC.

The same async feel works without BBS infrastructure. Put the campaign
directory on a shared VPS or any machine with SSH access. Players submit turns
on their own schedule, and a sysop runs maintenance when the year closes. Same
game, same rhythm, no door framework required.

You can also play solo or hotseat on your own machine. Create a campaign,
submit turns for one or more empires, and run maintenance locally. No network,
no server — just you and your terminal.

The Rust client is cross-platform, built on crossterm, and runs on Linux,
macOS, and Windows. Standalone packages are planned so you do not need a Rust
toolchain to play. Until those ship, the [Quick Start](#quick-start) commands
work with `cargo run` for anyone with a Rust environment.

EC does not show you everything on one screen. The game exports your starmap
as a printable text file and a companion CSV. Many players print the map or
pull the CSV into a spreadsheet to track fleet positions, mark explored
systems, and plan routes by hand. The Rust client has an interactive map
viewer in the TUI, but pencil on a printed starmap is the old-school way and
still a good one.

## Learn How To Play

The player manual covers everything from quick-start basics through economy,
combat, fleet missions, and strategy:

- **[EC Player Manual (PDF)](docs/manuals/ec_player_manual.pdf)**

Sysops setting up and administering a campaign — whether on a BBS, a shared
SSH host, or localhost — should also read:

- **[EC Sysop Manual (PDF)](docs/manuals/ec_sysop_manual.pdf)**

The original `.DOC` files are still preserved in [original/v1.5](original/v1.5).

## Background

EC is a Rust reimplementation of the original Esterian Conquest, with classic
`.DAT` interoperability, a growing native client, and a maintenance engine
that can already support serious campaign testing. The project aims to carry
the game forward without discarding what made it distinct: the yearly turn
rhythm, the empire reports, the starmap drama, the asymmetrical scouting and
warfare, and the old BBS command feel.

The immediate goal is a modern drop-in replacement for the classic door stack.
A canonical Rust game engine uses SQLite-native runtime state. An explicit
compatibility bridge handles classic `.DAT` import and export. The CLI
provides sysop, admin, and compatibility tooling, and a Rust player client is
intended to replace the original `ECGAME`.

The project is well past the stage of being a repo of notes and recovery
experiments. Fresh Rust-backed campaigns can be created across all four
documented player tiers (4, 9, 16, and 25 empires), and yearly turns run
through a real Rust maintenance engine. The growing native client already
handles substantial parts of a campaign, and classic `.DAT` interoperability
is preserved throughout. The original manuals and binaries remain available as
compatibility and historical references rather than as the center of
day-to-day development. It is not finished enough to call the reimplementation
complete end to end, but it is usable for real play, campaign validation, and
development testing today.

The architecture is converging on a full Rust-first stack. The shared state
model lives in `ec-data`, gameplay and maintenance rules live in `ec-engine`,
and an explicit `ec-compat` crate handles classic `.DAT` import and export so
the compatibility boundary stays clean. On top of that, `maint-rust` processes
turns, and `ec-client` is the growing player interface. Classic file
interchange is treated as a compatibility edge rather than the core runtime
path. That future state still respects the original game — the DOS binaries,
manuals, and data formats remain the primary reference for rules,
compatibility, and historical feel.

**[Read the Grand Vision: From BBS to the Decentralized Web](docs/grand-vision.md)**

**[How EC Was Recovered](docs/reverse_engineering/README.md)**

## Compatibility And Provenance

Compatibility is a first-class engineering goal, but it serves the Rust
reimplementation rather than replacing it. The
[player manual](docs/manuals/ec_player_manual.pdf) is the gameplay guide, the
original DOS binaries are the compatibility oracle, and classic `.DAT` files
remain the interchange boundary between the two worlds. Where the original
implementation was hidden, stochastic, or plainly buggy, Rust is allowed to be
explicit and reproducible instead. The heavy reverse-engineering phase is now
closed for normal development; the oracle stack remains in place as a
compatibility and regression backstop.

This project does not treat strict byte-for-byte historical reproduction as
the goal. Rust uses its own seeded combat system instead of the original
hidden RNG, and all engine randomness is rooted in a persisted campaign seed
so that results are reproducible. The Rust client may derive cosmetic
presentation choices from that same seed, but those never affect gameplay or
turn outcomes. Campaign-end handling is conservative and explicit rather than
opaque, and report wording is Rust-native where exact original text is not
required for compatibility. These differences are allowed by the project
approach as long as the result stays faithful to the manuals and compatible
with the original `.DAT` boundary.

For the detailed rationale, see [docs/approach.md](docs/dev/approach.md).

## Quick Start

Create a new game:

```bash
cd rust
cargo run -q -p ec-cli -- sysop new-game /tmp/ec-game --players 4 --seed 1515
```

This default path now creates a joinable pre-player `ECGAME` start with
inactive player slots and `Not Named Yet` homeworld seeds.

Run Rust maintenance:

```bash
cd rust
cargo run -q -p ec-cli -- maint-rust /tmp/ec-game 3
```

`maint-rust` now reads and writes the campaign's `ecgame.db`. Classic `.DAT`
directories are imported/exported through the CLI compatibility bridge.
`maint-rust` does not project the latest snapshot back into the working
directory automatically. Use `db-export` when you intentionally want classic
`.DAT` output through the explicit compatibility bridge for oracle runs or DOS
`ECGAME`, and use `db-import` if classic tools changed the directory and you
want SQLite to pick up those edits.

Run the original oracle against that directory:

```bash
python3 tools/ecmaint_oracle.py run /tmp/ec-game
```

Launch original `ECGAME` locally in DOSBox-X:

```bash
tools/run_ecgame.sh /tmp/ec-game 1
```

For local returning-player probes, pass a caller alias that matches the
persisted player handle:

```bash
tools/run_ecgame.sh /tmp/ec-game 2 SYSOP
```

Build the reproducible demo-ready release zips for emulator testing:

```bash
python3 scripts/build_release_packages.py --verify
```

Publish the current release assets in one step:

```bash
./scripts/publish_release_packages.sh
```

This writes:

- `releases/ec-v1.5-classic-demo.zip`
- `releases/ec-v1.5-unlocked-demo.zip`

These zip files are local build output and are kept untracked in `main`.
Published copies live as GitHub release assets under the repo's Releases page.

| Bundle | DOSBox-X | dosemu2 | Notes |
|---|---|---|---|
| `releases/ec-v1.5-classic-demo.zip` | Verified | Not currently working | Original packed/oracle bundle; 8s smoke pass from `/tmp` with the known-good local-console `CHAIN.TXT`. |
| `releases/ec-v1.5-unlocked-demo.zip` | Verified | Not currently working | Curated runnable plain-MZ bundle; 8s smoke pass from `/tmp` after the `ECGAME.EXE` MZ size-field fix. |

Each archive includes a minimal real game directory, a known-good local
`CHAIN.TXT`, and the original `.DOC` manuals. Here, `Verified` means the
package survived the repo's DOSBox-X smoke launch without the old INT 6 / GPF
failures. DOSBox-X is currently the only verified local runner for the
original EC v1.5 binaries.

The unlocked bundle rebuilds `EC_UNLOCKED/` first, including the current
`ECGAME.EXE` recovery that corrects the memdump image's MZ size fields so DOS
loads the full unlocked client body.

Inspect the classic login branch Rust expects for a given caller alias:

```bash
cd rust
cargo run -q -p ec-cli -- inspect-classic-login /tmp/ec-game SYSOP
```

Prepare a player slot for a local matched-alias classic probe:

```bash
cd rust
cargo run -q -p ec-cli -- classic-login-prepare /tmp/ec-game 2 SYSOP foo
```

Submit a player turn from KDL:

```bash
cd rust
cargo run -q -p ec-cli -- submit-turn --check --dir /tmp/ec-game --player 1 --file /tmp/player1-turn.kdl
cargo run -q -p ec-cli -- submit-turn --dir /tmp/ec-game --player 1 --file /tmp/player1-turn.kdl
```

The turn file format is documented in [docs/player/turn-kdl.md](docs/player/turn-kdl.md).

Build a runtime playtest scenario from KDL:

```bash
cd rust
cargo run -q -p ec-cli -- harness run-scenario --file /tmp/scenario.kdl --dir /tmp/ec-scenario
```

Bootstrap a multi-bot campaign and stop with turn 5 open for TUI inspection:

```bash
cd rust
cargo run -q -p ec-cli -- harness play-until --file /tmp/scenario.kdl --dir /tmp/ec-bot-campaign --game-id tui-polish --turn 5
```

If the conductor blocks on missing or invalid player turns, fill the required
`.tmp/llm-turns/<game_id>/player-<n>/turn-<nnnn>.kdl` files and rerun the same
command. The coordinator workflow and bot-safe workspace layout are documented
in [docs/dev/harness/campaign-play.md](docs/dev/harness/campaign-play.md) and
[docs/dev/llm-player-guide.md](docs/dev/llm-player-guide.md).

Run a combat sweep from KDL:

```bash
cd rust
cargo run -q -p ec-cli -- harness run-sweep --file /tmp/combat-sweep.kdl
```

The scenario/combat harness format is documented in [docs/dev/harness/README.md](docs/dev/harness/README.md).

Supported local hybrid loop:

```bash
cd rust
cargo run -q -p ec-cli -- sysop new-game /tmp/ec-game --players 4 --seed 1515
cargo run -q -p ec-cli -- inspect-classic-login /tmp/ec-game SYSOP
../tools/run_ecgame.sh /tmp/ec-game 1
cargo run -q -p ec-cli -- classic-login-prepare /tmp/ec-game 1 SYSOP foo
cargo run -q -p ec-cli -- db-export /tmp/ec-game /tmp/ec-game
../tools/run_ecgame.sh /tmp/ec-game 1 SYSOP
cargo run -q -p ec-cli -- maint-rust /tmp/ec-game 1
cargo run -q -p ec-cli -- db-export /tmp/ec-game /tmp/ec-game
../tools/run_ecgame.sh /tmp/ec-game 1 SYSOP
```

This is a supported local compatibility loop for classic `ECGAME` on top of
Rust maintenance. It does not claim byte-faithful classic `MESSAGES.DAT`
reproduction; current Rust behavior preserves existing classic player mail and
maintains classic-readable results/report files.

Known deliberate divergence:

- original `ECMAINT` has a regular-world `ScoutSolarSystem` lone-active-mission
  abort bug; `maint-rust` documents that oracle behavior but does not copy it
- the recovered successful foreign-world scout refresh family is tied to a
  legacy rogue-viewer campaign state in original `ECMAINT`; `maint-rust` keeps
  explicit active-player foreign-intel refresh semantics instead of emulating
  that state quirk

The classic login classifier now covers all three local compatibility branches:

- `first-time-menu`
- `matched-preloaded-first-login`
- `returning-player`

Run the Rust client:

```bash
cd rust
cargo run -q -p ec-client -- --dir /tmp/ec-game --player 1
```

`ec-client` now loads campaign state from `ecgame.db`. Fresh Rust-created games
seed that DB automatically. If you mutate a classic directory outside the
SQLite path, run `db-import` before launching the client or `maint-rust`.

Test harness scripts live under [scripts/](scripts/):

```bash
python3 scripts/new_test_game.py /tmp/ec-join-test --players 9 --force
python3 scripts/setup_ui_stress_game.py /tmp/ec-ui-stress --force
python3 scripts/setup_classic_probe_game.py /tmp/ec-classic-probe --force --no-launch
python3 scripts/run_client.py /tmp/ec-ui-stress --player 1
```

For the original DOS client specifically, the fastest "busy campaign" probe is:

```bash
python3 scripts/setup_classic_probe_game.py /tmp/ec-classic-probe --force
```

Or use the thin wrapper for the common report-format probe:

```bash
./scripts/run_classic_report_probe.sh
```

That path creates a fresh four-player Rust-backed campaign, seeds player 1 with
multiple fleets and worlds, runs several Rust maint turns to populate
`RESULTS.DAT`, preserves classic-compatible `MESSAGES.DAT` state for
classic compatibility, prepares the classic login alias, and then
launches original `ECGAME` in DOSBox-X.

## Useful Commands

New game from declarative config:

```bash
cd rust
cargo run -q -p ec-cli -- sysop new-game /tmp/ec-game --config ec-data/config/setup.example.kdl
```

The Rust client now uses a built-in ASCII splash followed by the in-client intro
pages.

Inspect a game directory:

```bash
cd rust
cargo run -q -p ec-cli -- core-report /tmp/ec-game
```

Inspect classic player mail:

```bash
cd rust
cargo run -q -p ec-cli -- inspect-messages /tmp/ec-game
```

Export a player-safe printable starmap and companion CSV:

```bash
cd rust
cargo run -q -p ec-cli -- map-export /tmp/ec-game 1 /tmp/ec-exports/ECMAP-P1-Y3000.TXT
```

Import a classic game directory into the bundled per-campaign SQLite store:

```bash
cd rust
cargo run -q -p ec-cli -- db-import /tmp/ec-game
```

Export the latest `ecgame.db` snapshot back to a classic-compatible directory:

```bash
cd rust
cargo run -q -p ec-cli -- db-export /tmp/ec-game /tmp/ec-game-exported
```

The intended boundary is:

- `ec-client` and `maint-rust` operate on `ecgame.db`
- `db-import` and `db-export` are the explicit classic `.DAT` bridge

Run the broader validation sweeps:

```bash
python3 tools/oracle_sweep.py --mode seeded
python3 tools/rust_maint_sweep.py --turns 3
```

## Local Dependencies

For normal Rust development in this repo, the practical baseline is:

- Rust toolchain with `cargo`
- Python 3 for oracle/support scripts under `tools/`
- `python-pexpect` if you want to use the DOSBox-X debugger helpers under
  `tools/`
- DOSBox-X if you want to launch the original DOS binaries locally or do
  targeted compatibility/provenance work (`EC_UNLOCKED/` holds the stub-free
  local-launch set, but DOSBox-X is currently the only verified runner for EC
  v1.5)
- Ghidra plus JDK 21 only if you want the headless static-analysis workflow

Recommended local build-speed tooling:

- `sccache`

On Arch-based systems:

```bash
sudo pacman -S sccache python-pexpect dosbox-x ghidra jdk21-openjdk
```

If you use the packaged Arch/CachyOS Ghidra build, the practical repo settings
are:

```bash
export GHIDRA_HOME=/opt/ghidra
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk
```

Then enable it in your local Cargo config:

```toml
[build]
rustc-wrapper = "sccache"
```

Notes:

- Cargo already uses multiple cores by default; there is no repo-local
  `jobs = ...` override checked in here.
- `sccache` is the preferred low-risk speedup for this workspace.
- `mold` can help on some systems, but it is not required by the repo and is
  not currently recommended as a documented baseline dependency.

## Read First

- [docs/approach.md](docs/dev/approach.md)
- [docs/next-session.md](docs/dev/next-session.md)
- [docs/rust-architecture.md](docs/dev/rust-architecture.md)

Useful supporting docs:

- [docs/ec-setup-spec.md](docs/dev/ec-setup-spec.md)
- [docs/economics.md](docs/dev/economics.md)
- [docs/ec-combat-spec.md](docs/dev/ec-combat-spec.md)
- [docs/ec-movement-spec.md](docs/dev/ec-movement-spec.md)
- [docs/reverse_engineering/README.md](docs/reverse_engineering/README.md)
- [docs/bbs_door_client_rust.md](docs/dev/bbs_door_client_rust.md)
- [docs/sysop-map-exports.md](docs/sysop/sysop-map-exports.md)
- [docs/dosbox-workflow.md](docs/dev/dosbox-workflow.md)

## Repository Layout

- `original/`: original EC 1.5 materials used as primary sources and oracle
  artifacts
- `EC_UNLOCKED/`: curated runnable plain-MZ copies of the original DOS executables
- `docs/`: stable engineering, RE, and design docs
- `docs/reverse_engineering/`: oracle, provenance, and binary-recovery docs
- `docs/dev/archive/RE_NOTES.md`: chronological reverse-engineering notebook (archival)
- `rust/ec-data`: shared runtime/store/state-model crate
- `rust/ec-classic`: low-level classic record/codec support crate
- `rust/ec-engine`: public gameplay/maintenance/rules crate
- `rust/ec-compat`: classic `.DAT` import/export bridge
- `rust/ec-cli`: sysop/admin/oracle/inspection CLI
- `rust/ec-client`: Rust `ECGAME` replacement in active development
- `tools/`: oracle runners, DOSBox helpers, and analysis scripts

## License

The new source code and tooling in this repository are licensed under the MIT
License. See [LICENSE](LICENSE).

The original Esterian Conquest DOS binaries, data files, manuals, logs, and
other preserved game materials remain original works of Bently C. Griffith and
their original rights holders. Their inclusion here is for preservation,
research, and compatibility work; they are not relicensed under MIT by this
repository.
