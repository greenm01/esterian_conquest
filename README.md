# esterian_conquest

![Esterian Conquest banner](capture/ec_v1.5_banner.png)

Rust resurrection of Esterian Conquest v1.5, with classic `.DAT`
interoperability, a growing native client, and a maintenance engine nearing
full compliance.

**[Read the Grand Vision: From BBS to the Decentralized Web](docs/grand-vision.md)**

This project started as a file-format and reverse-engineering effort. It is now
past that stage: the Rust side is being built as a full replacement stack for
Phase 1 of the grand vision, the modern drop-in BBS door replacement:

- a canonical Rust game engine with SQLite-native runtime state
- a Rust sysop/admin/oracle toolchain
- a Rust player client intended to replace `ECGAME`

The engine is already strong enough for serious campaign testing and hybrid
play while remaining accepted by the current oracle/toolchain checks. Full
Rust maint compliance is still being finished, and the client phase is
actively underway.

## Learn How To Play

If you are new to Esterian Conquest, start with the readable manual
transcriptions first:

- [ECQSTART.md](docs/manuals/ECQSTART.md)
  - quick-start overview
- [ECPLAYER.md](docs/manuals/ECPLAYER.md)
  - the main player manual
- [ECREADME.md](docs/manuals/ECREADME.md)
  - setup notes, release notes, and bundled guidance
- [WHATSNEW.md](docs/manuals/WHATSNEW.md)
  - version-history changes for late-classic EC

Full manual index:
- [docs/manuals/README.md](docs/manuals/README.md)

The original `.DOC` files are still preserved in [original/v1.5](original/v1.5).

Those manuals are not just historical reference. They are the canonical
player-facing guide for how the game is supposed to work.

## What You Can Do Today

This is no longer just a preservation repo or an academic RE exercise.
It is also not finished enough yet to call the Rust replacement fully playable
end to end.

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

## How EC Was Recovered

The Rust version was not built from guesswork. The current engine and docs came
from repeated cross-checking between the original manuals, the original DOS
binaries, preserved fixtures, and Rust-generated controlled scenarios.

| Tool / source | What it was used for | Why it mattered |
| --- | --- | --- |
| Original EC manuals in [`original/v1.5/*.DOC`](original/v1.5) | Treated as the canonical guide for player-facing rules, setup constraints, turn structure, and terminology | Kept the Rust clone grounded in intended game behavior instead of raw binary quirks alone |
| Ghidra disassembly and headless scripts | Static recovery of file layouts, maintenance flow, scheduler logic, and helper call structure | Let us turn opaque Pascal-era code paths into stable Rust-facing specs |
| DOSBox-X debugger, INT 21 tracing, and memory dumps | Dynamic tracing of `ECGAME` / `ECMAINT` behavior, file I/O order, token handling, and live state changes | Proved phase ordering, runtime state transitions, and report/output boundaries that static RE alone could not settle |
| Controlled gamestate file diffs | Compared Rust-generated or hand-shaped directories against classic `.DAT` outputs before and after maintenance | Exposed real cross-file invariants and kept the Rust side honest at the compatibility boundary |
| Extensive report and log analysis | Studied `RESULTS.DAT`, `MESSAGES.DAT`, shipped `ec*.txt` logs, and preserved output captures | Recovered player-visible timing, report cadence, `Stardate` behavior, and event sequencing |
| Rust-generated scenarios and oracle sweeps | Created narrow test cases, ran the original binaries as oracle, and promoted repeated outcomes into shared rules | Turned reverse engineering into reusable implementation guidance instead of one-off notes |

The project rule that fell out of that work is simple:

- the manuals are the gameplay guide
- the original binaries are the compatibility oracle
- the Rust side is allowed to be explicit, deterministic, and testable where
  the original implementation was hidden or stochastic

## Current Status

The project is in a mixed engine-near-complete / client-in-progress phase.

Today the Rust side can:

- generate joinable new games across the documented `4 / 9 / 16 / 25` player
  tiers
- run repeated maintenance turns through the current Rust engine
- handle movement, economy, scouting, diplomacy, conquest, civil disorder,
  fleet defection, and campaign-end recognition
- write classic-compatible `PLAYER.DAT`, `PLANETS.DAT`, `FLEETS.DAT`,
  `CONQUEST.DAT`, `SETUP.DAT`, `DATABASE.DAT`, `RESULTS.DAT`
- preserve classic player mail in `MESSAGES.DAT`
- produce directories the original `ECMAINT` still accepts
- persist active campaigns in bundled `ecgame.db`
- run `ec-client` and `maint-rust` against SQLite runtime state instead of
  directly mutating classic `.DAT` files
- keep classic `.DAT` import/export at the CLI boundary instead of inside the
  player TUI or Rust maintenance runtime
- create default `sysop new-game` directories that `ECGAME` can actually join
  through the original onboarding flow
- export the latest SQLite-backed snapshot to classic `.DAT` only when an
  explicit `db-export` or classic materialization workflow asks for it
- import classic `.DAT` edits back into SQLite with `db-import` when DOS
  tooling changed a directory and Rust should resume from that compatibility
  state
- enforce major player-input legality in the shared Rust engine instead of
  trusting the player client:
  - fleet orders and mission payloads
  - fleet ROE, speed, and transport-army consistency
  - build queue and stardock payloads
  - tax-rate inputs
  - stored diplomacy inputs
  - commission ownership / stardock actions
- sanitize malformed player-authored state during `maint-rust` instead of
  panicking or silently executing illegal effects
- emit maintenance reports when invalid player input is canceled or corrected
- provide a growing Rust player client with working startup flow and real
  command-center coverage for:
  - General Command
  - Planet Command
  - most of Fleet Command, including starbase and fleet-order workflows
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
- `rust-maint` is close to a real drop-in campaign engine, but final
  compliance work is still active
- the maintenance engine is now the authority for the major gameplay inputs it
  consumes; frontend validation is convenience, not the trust boundary
- the Rust client is no longer speculative UI work; it is actively replacing
  `ECGAME` screen by screen and command by command
- the runtime architecture has already crossed the important boundary:
  `ec-client` and `maint-rust` are SQLite-native, while `ec-cli` remains the
  explicit `.DAT` compatibility bridge

Maintenance engine status, approximately:

- core turn-cycle, timing, combat, movement, and economy specs are now closed
- Rust maint compliance against those recovered rules is the main remaining
  engine task
- the engine is close enough for serious testing and hybrid play, but not yet
  finished enough to call complete

That does not mean every client/menu path is finished. It means the engine is
now in the shape we need for Phase 1: shared rules live in `ec-data`, malformed
player state is audited and sanitized in maint, and the CLI path has
deterministic malformed-directory stress coverage.

Player client status, approximately:

- engine / maintenance / storage: strong enough to support real client work,
  with maint compliance still being finished
- player TUI: roughly `85%` of the classic v1.5 feature surface is now present
  in some usable form
- remaining work is less about proving the architecture and more about:
  - finishing the remaining command/menu paths
  - tightening fidelity against v1.5 screens and behaviors
  - fixing UI bugs, picker edge cases, and other terminal polish
  - completing the BBS-door-facing flow around the local TUI

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

## Current Focus

The engine/admin side is now strong enough that the main implementation focus
has shifted from:

- "can Rust maintain a game?"

to:

- "can Rust replace `ECGAME` well?"

Current emphasis:

- keep `rust-maint` honest with continued oracle sweeps
- finish the remaining Rust player command/menu surfaces
- keep Phase 1 aligned with the grand vision:
  - faithful classic mechanics
  - fixed terminal playfield
  - modern-host-friendly native Rust deployment
- preserve classic terminology and workflow where it helps
- modernize only where the original UI was clearly hostile or obsolete
  (for example map export and terminal-safe compose flows)
- build the local terminal client first, then carry that into BBS door support
- keep startup presentation inside the fixed client playfield:
  - built-in ASCII splash
  - paged in-client intro text
  - no raw ANSI dump path during normal `ec-client` startup

That client work is now documented in
[docs/bbs_door_client_rust.md](docs/dev/bbs_door_client_rust.md).

Player map delivery and sysop staging are documented in
[docs/sysop-map-exports.md](docs/sysop/sysop-map-exports.md).

## Local Dependencies

For normal Rust development in this repo, the practical baseline is:

- Rust toolchain with `cargo`
- Python 3 for oracle/support scripts under `tools/`
- DOSBox-X only if you want to launch the original DOS binaries locally

Recommended local build-speed tooling:

- `sccache`

On Arch-based systems:

```bash
sudo pacman -S sccache
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
`RESULTS.DAT`, suppresses the Rust-only routed `MESSAGES.DAT` payload for
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
- [docs/bbs_door_client_rust.md](docs/dev/bbs_door_client_rust.md)
- [docs/sysop-map-exports.md](docs/sysop/sysop-map-exports.md)
- [docs/dosbox-workflow.md](docs/dev/dosbox-workflow.md)

## Repository Layout

- `original/`: original EC 1.5 materials used as primary sources and oracle
  artifacts
- `docs/`: stable engineering, RE, and design docs
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
