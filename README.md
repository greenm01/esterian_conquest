# esterian_conquest

![Esterian Conquest banner](capture/ec_v1.5_banner.png)

Rust preservation and reimplementation work for Esterian Conquest v1.5.

This project started as a file-format and reverse-engineering effort. It is now
past that stage: the Rust side is being built as a full replacement stack:

- a canonical Rust game engine and `.DAT` state layer
- a Rust sysop/admin/oracle toolchain
- a Rust player client intended to replace `ECGAME`

The engine is already strong enough to run full campaigns while remaining
accepted by the original DOS toolchain under the project's actual
compatibility standard. The client phase is now actively underway.

## Premise

Beyond the mapped frontiers of the old Esterian dominion lies a small galaxy
of contested solar systems. The old masters are gone. Their stations are
silent, their patrols vanished, and their subjects left with fleets,
factories, and enough knowledge to build empires.

`v1.6` treats that classic premise seriously. The goal is not to reboot the
game into something unrecognizable, but to carry Esterian Conquest forward:
preserve its campaign feel, menus, reports, and BBS-era drama while replacing
the original DOS constraints with a modern Rust implementation.

## What This Repo Is Doing

Three things at once:

- preserving the original DOS game, manuals, logs, and binaries
- reverse engineering the rules and on-disk formats
- building a modern Rust replacement without giving up classic `.DAT`
  interoperability

The key project rule is simple:

- the manuals are the gameplay spec
- the original binaries are the compatibility oracle
- the classic game directory is still the interchange boundary

That does not mean "byte-identical to one historical run." It means the Rust
side must remain loadable, sane, and acceptable to the original tools while
allowing documented canonical Rust behavior where the original internals are
hidden, stochastic, or not worth cloning literally.

## Project Intent

The long-term goal is not just "Rust tools around the old binaries."

The intended end state is:

- `ec-data` as the canonical Rust engine and state model
- `ec-cli` as the sysop/admin/oracle surface
- `ec-client` as a full Rust `ECGAME` replacement
- classic `.DAT` interoperability kept as the compatibility boundary
- the original manuals treated as the gameplay spec
- the original binaries treated as the compatibility oracle

In plain terms: this project is aiming at a complete Rust engine/client
replacement for Esterian Conquest, while preserving the original campaign
feel, data layout, and BBS-era workflow where those still matter.

## Current Status

The project is in a mixed engine-complete / client-in-progress phase.

Today the Rust side can:

- generate joinable new games across the documented `4 / 9 / 16 / 25` player
  tiers
- run repeated maintenance turns through the Rust engine
- handle movement, economy, scouting, diplomacy, conquest, civil disorder,
  fleet defection, and campaign-end recognition
- write classic-compatible `PLAYER.DAT`, `PLANETS.DAT`, `FLEETS.DAT`,
  `CONQUEST.DAT`, `SETUP.DAT`, `DATABASE.DAT`, `RESULTS.DAT`
- preserve classic player mail in `MESSAGES.DAT`
- produce directories the original `ECMAINT` still accepts
- create default `sysop new-game` directories that `ECGAME` can actually join
  through the original onboarding flow
- provide a growing Rust player client with working startup flow and real
  command-center coverage for:
  - General Command
  - Planet Command
  - map export / starmap viewing
  - diplomacy, mail, commissioning, build, and transport flows

Recent validation:

- `python3 tools/oracle_sweep.py --mode seeded`
  - current result: `12/12` passes
- `python3 tools/rust_maint_sweep.py --turns 3`
  - current result: `8/8` passes
- `cargo test -q`
  - current workspace status: green

In plain terms:

- Rust is no longer just a scenario generator or fixture toy
- `rust-maint` is usable as a real campaign engine
- the Rust client is no longer speculative UI work; it is actively replacing
  `ECGAME` screen by screen and command by command

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

For the detailed rationale, see [docs/approach.md](docs/approach.md).

## Current Focus

The engine/admin side is now strong enough that the main implementation focus
has shifted from:

- "can Rust maintain a game?"

to:

- "can Rust replace `ECGAME` well?"

Current emphasis:

- keep `rust-maint` honest with continued oracle sweeps
- finish the remaining Rust player command/menu surfaces
- preserve classic terminology and workflow where it helps
- modernize only where the original UI was clearly hostile or obsolete
  (for example map export and terminal-safe compose flows)
- build the local terminal client first, then carry that into BBS door support
- keep startup art asset-driven:
  - BBS splash and EC splash sources now come from KDL-configured files
  - the Rust client still owns pacing, 80x20 rendering, and flow control

That client work is now documented in
[docs/bbs_door_client_rust.md](docs/bbs_door_client_rust.md).

Player map delivery and sysop staging are documented in
[docs/sysop-map-exports.md](docs/sysop-map-exports.md).

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

Run the original oracle against that directory:

```bash
python3 tools/ecmaint_oracle.py run /tmp/ec-game
```

Launch original `ECGAME` locally in DOSBox-X:

```bash
tools/run_ecgame.sh /tmp/ec-game 1
```

Run the Rust client:

```bash
cd rust
cargo run -q -p ec-client -- --dir /tmp/ec-game --player 1
```

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

Run the broader validation sweeps:

```bash
python3 tools/oracle_sweep.py --mode seeded
python3 tools/rust_maint_sweep.py --turns 3
```

## Read First

- [docs/approach.md](docs/approach.md)
- [docs/next-session.md](docs/next-session.md)
- [docs/rust-architecture.md](docs/rust-architecture.md)

Useful supporting docs:

- [docs/ec-setup-spec.md](docs/ec-setup-spec.md)
- [docs/economics.md](docs/economics.md)
- [docs/ec-combat-spec.md](docs/ec-combat-spec.md)
- [docs/ec-movement-spec.md](docs/ec-movement-spec.md)
- [docs/bbs_door_client_rust.md](docs/bbs_door_client_rust.md)
- [docs/sysop-map-exports.md](docs/sysop-map-exports.md)
- [docs/dosbox-workflow.md](docs/dosbox-workflow.md)

## Repository Layout

- `original/`: original EC 1.5 materials used as primary sources and oracle
  artifacts
- `docs/`: stable engineering, RE, and design docs
- `RE_NOTES.md`: chronological reverse-engineering notebook
- `rust/ec-data`: canonical Rust state/model/engine crate
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
