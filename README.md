# esterian_conquest

![Esterian Conquest banner](docs/assets/ec_v1.5_banner.png)

_Inspired by Esterian Conquest (c) 1992 Bentley C. Griffith.
A fan-built resurrection -- not affiliated with the original._

EC is a Rust reimplementation of the original game, with classic `.DAT`
interoperability, a growing native client, and a maintenance engine that can
already support serious campaign testing.

Esterian Conquest is an asynchronous turn-based strategy game: players submit
orders during the year, and maintenance resolves the turn.

**[Read the Grand Vision: From BBS to the Decentralized Web](docs/grand-vision.md)**

**[How EC Was Recovered](docs/reverse_engineering/README.md)**

EC aims to carry Esterian Conquest forward without discarding what made the
original game distinct: the yearly turn rhythm, the empire reports, the starmap
drama, the asymmetrical scouting and warfare, and the old BBS command feel.

The immediate goal is a modern drop-in replacement for the classic door stack,
with:

- a canonical Rust game engine with SQLite-native runtime state
- a Rust sysop/admin/oracle toolchain
- a Rust player client intended to replace `ECGAME`

The current project state is practical rather than speculative. The Rust engine
can already create campaigns, process maintenance, interoperate with classic
directories, and support hybrid play with the original DOS client where that is
still useful.

## Premise

Beyond the mapped frontiers of the old Esterian dominion lies a small galaxy
of contested solar systems. The old masters are gone. Their stations are
silent, their patrols vanished, and their subjects left with fleets,
factories, and enough knowledge to build empires.

EC treats that classic premise seriously. The goal is not to sand away the
identity of the original game, but to keep its campaign feel, menus, reports,
and old-school tension while replacing the DOS runtime with a modern Rust
implementation.

## Learn How To Play

If you are new to Esterian Conquest, start with the readable manual
transcriptions first:

- [ec_qstart.md](docs/manuals/ec_qstart.md)
  - quick-start overview
- [ec_player.md](docs/manuals/ec_player.md)
  - the main player manual
- [ec_readme.md](docs/manuals/ec_readme.md)
  - setup notes, release notes, and bundled guidance

Full manual index:
- [docs/manuals/README.md](docs/manuals/README.md)

The original `.DOC` files are still preserved in [original/v1.5](original/v1.5).

Those manuals are not just historical reference. They are the canonical
player-facing guide for how the game is supposed to work.

## Current State

EC is already usable for real development play, hybrid classic play, and
campaign validation. It is not finished enough yet to call the Rust reimplementation
complete end to end, but it is well past the stage of being a repo of notes and
recovery experiments.

Today you can:

- create fresh Rust-backed campaigns across the documented `4 / 9 / 16 / 25`
  player tiers
- run yearly turns through the current Rust maintenance engine while compliance
  work continues
- play substantial parts of those campaigns through the growing native Rust
  client
- keep using the original DOS `ECGAME` in a supported hybrid loop when you want
  classic order entry and viewing
- preserve classic `.DAT` interoperability while the Rust engine and client
  continue taking over more of the game
- validate Rust-generated directories and turn behavior against the original
  manuals and binaries

If you want to jump in immediately, start with [Quick Start](#quick-start).

## Where EC Is Going

The project is moving toward a full Rust-first Esterian Conquest stack:

- `ec-data` as the canonical game engine and state model
- `maint-rust` as the normal turn processor
- `ec-client` as the normal player interface
- classic `.DAT` import/export as a compatibility boundary instead of the core
  runtime

That future state still respects the original game. The DOS binaries, manuals,
and data formats remain the primary reference for rules, compatibility, and
historical feel.

## Compatibility And Provenance

Compatibility remains a first-class engineering goal, but it is in service of
the Rust reimplementation, not a substitute for it.

Short version:

- the manuals in `original/v1.5/*.DOC` are the gameplay guide
- the original DOS binaries are the compatibility oracle
- classic `.DAT` files remain the interchange boundary
- Rust is allowed to be explicit and deterministic where the original
  implementation was hidden, stochastic, or plainly buggy

## Where Rust Intentionally Differs

This project does not treat strict historical byte-for-byte reproduction as the
goal.

Known intentional differences include:

- deterministic Rust combat instead of the original hidden RNG
- conservative explicit campaign-end handling
- Rust-native report wording where exact original text is not required for
  compatibility

Those differences are allowed by the project approach as long as the result
remains faithful to the manuals and compatible with the original `.DAT`
boundary.

For the detailed rationale, see [docs/approach.md](docs/dev/approach.md).

## Local Dependencies

For normal Rust development in this repo, the practical baseline is:

- Rust toolchain with `cargo`
- Python 3 for oracle/support scripts under `tools/`
- `python-pexpect` if you want to use the DOSBox-X debugger helpers under
  `tools/`
- DOSBox-X or dosemu2 if you want to launch the original DOS binaries locally
  or do dynamic oracle/RE work (the unlocked binaries in `EC_UNLOCKED/` work
  in both; the original packed binaries require DOSBox-X)
- Ghidra plus JDK 21 only if you want to use the headless static-RE workflow

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
`.DAT` output for oracle runs or DOS `ECGAME`, and use `db-import` if classic
tools changed the directory and you want SQLite to pick up those edits.

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

For the original DOS client specifically, the fastest “busy campaign” probe is:

```bash
python3 scripts/setup_classic_probe_game.py /tmp/ec-classic-probe --force
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

The bundled example config uses `setup_mode="builder-compatible"` for the
active-campaign baseline used by the maint/oracle sweeps.

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
- `EC_UNLOCKED/`: decrypted, runnable copies of the original DOS executables
- `docs/`: stable engineering, RE, and design docs
- `docs/reverse_engineering/`: oracle, provenance, and binary-recovery docs
- `docs/dev/archive/RE_NOTES.md`: chronological reverse-engineering notebook (archival)
- `rust/ec-data`: canonical Rust state/model/engine crate
- `rust/ec-cli`: sysop/admin/oracle/inspection CLI
- `rust/ec-client`: Rust `ECGAME` replacement in active development
- `tools/`: oracle runners, DOSBox helpers, and analysis scripts

## Contributions Welcome

This project is now at the point where outside help is genuinely useful.

High-value contributions right now:

- playtesters who are willing to run real campaigns or focused turn-by-turn
  probes and report where the Rust client or Rust maint still feels wrong
- ANSI / CP437 artists who can help preserve the classic BBS mood while giving
  the Rust client cleaner splash screens, framing, and in-client presentation
- terminal UI polish work, especially around menus, layout, readability, and
  classic-feeling presentation that still works well on modern terminals

If you know classic BBS games, ANSI presentation, or just want to help beat on
the near-finished Rust maintenance/client loop, this is a good time to jump in.

## License

The new source code and tooling in this repository are licensed under the MIT
License. See [LICENSE](LICENSE).

The original Esterian Conquest DOS binaries, data files, manuals, logs, and
other preserved game materials remain original works of Bently C. Griffith and
their original rights holders. Their inclusion here is for preservation,
research, and compatibility work; they are not relicensed under MIT by this
repository.
