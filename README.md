# esterian_conquest

Preservation and reimplementation workspace for Esterian Conquest v1.5.

The project started as a file-format and oracle-compliance effort. It has now
crossed the more important line: `rust-maint` can run full Esterian Conquest
campaigns end to end while continuing to write classic-compatible `.DAT`
directories that the original DOS tools still accept.

## What This Project Is

This repository has three jobs:

- preserve the original DOS game, manuals, logs, and binaries
- reverse engineer the game rules and on-disk formats
- build a faithful modern Rust implementation without breaking classic save
  compatibility

The compatibility boundary is still the original game directory. Rust is not
trying to replace the `.DAT` files with a private format and call that done.
The point is to keep proving faithfulness against the original game.

## Current Status

The Rust side is no longer just a scenario generator.

Today it can:

- generate new classic-compatible games across the documented `4 / 9 / 16 / 25`
  player tiers
- generate default `sysop new-game` directories as joinable `ECGAME` starts
- run repeated maintenance turns through the Rust engine
- handle movement, economy, scouting, contact reports, diplomacy, deterministic
  combat, conquest, civil disorder, fleet defection, and conservative emperor
  recognition
- regenerate classic report and database files
- keep producing directories the original `ECMAINT` accepts

Recent end-to-end validation:

- seeded `sysop new-game` outputs pass the original `ECMAINT` oracle
- repeated `maint-rust` output after multiple turns also passes the original
  `ECMAINT` oracle
- current multi-turn sweep result: `8/8` passes across `4/9/16/25` players and
  seeds `1515/2025` for `3` Rust maint turns each

In practical terms: Rust maint can now run real EC games, not just one-turn
fixtures.

## Where Rust Intentionally Differs From The Oracle

This project does not treat byte-for-byte parity with one historical
`ECMAINT.EXE` run as the highest good.

We follow a stricter rule:

- manuals are the semantic authority
- DOS binaries are the compatibility oracle
- hidden or stochastic internals may be reimplemented canonically if the result
  stays faithful to the manuals and keeps classic gamestate compatibility

That matters most in these areas:

- combat is deterministic in Rust, not driven by the original hidden RNG
- campaign-end behavior is conservative and explicit in Rust rather than guessed
  from opaque binary state
- route planning is allowed to be smarter than undocumented original internals,
  while still respecting fog of war

### Combat

The original game uses internal randomness for fleet battles, bombardment
losses, and related resolution. Matching that exactly would require reproducing
hidden RNG state and fragile processing quirks, which is the wrong target for a
preservation-grade reimplementation.

So Rust uses a documented deterministic combat system instead.

The current combat model is deliberately influenced by *Empire of the Sun*:

- simultaneous exchange rather than brittle file-order firing
- clearer orbital and ground-combat phases
- deterministic bilateral loss accounting
- campaign reports that remain readable and auditable

That is an intentional design choice. We chose it because it produces stable,
testable outcomes while preserving the spirit of EC’s large-scale strategic
combat better than trying to mimic one opaque RNG stream.

See [docs/ec-combat-spec.md](docs/ec-combat-spec.md).

### Campaign-End Rules

The manuals talk about surrender, fleet defection, and recognition of an
emperor, but `ECGAME` does not expose a surrender command in the General
Command menu.

Rust therefore models campaign end as maintenance/state logic, not as an
invented UI action.

Current conservative rules:

- an empire with no planets and no recovery path falls into civil disorder
- once already in civil disorder and still planetless, it loses one fleet to
  defection per maintenance turn
- if exactly one serious contender remains and that empire is still stable and
  planet-owning, Rust recognizes it as emperor

These rules are documented, deterministic, and compatible. They can still be
refined later if stronger original evidence appears.

## Why `.DAT` Compatibility Still Matters

The project is only interesting if the Rust engine can keep proving itself
against the original game.

That means:

- `.DAT` compatibility remains mandatory
- original `ECMAINT` remains part of the validation story
- original `ECGAME` remains useful as a viewer and black-box check

Future storage work such as SQLite is still on the table, but only as an
additional Rust-native layer. It does not replace the classic game directory as
the compatibility boundary.

## Quick Start

Create a new game:

```bash
cd rust
cargo run -q -p ec-cli -- sysop new-game /tmp/ec-game --players 4 --seed 1515
```

This default path now creates a joinable pre-player `ECGAME` start with
inactive player slots and `Not Named Yet` homeworld seeds.

Run Rust maintenance for a few turns:

```bash
cd rust
cargo run -q -p ec-cli -- maint-rust /tmp/ec-game 3
```

Run the original oracle against that directory:

```bash
python3 tools/ecmaint_oracle.py run /tmp/ec-game
```

Run the broader sweeps:

```bash
python3 tools/oracle_sweep.py --mode seeded
python3 tools/rust_maint_sweep.py --turns 3
```

## Useful Commands

New game from declarative setup:

```bash
cd rust
cargo run -q -p ec-cli -- sysop new-game /tmp/ec-game --config ec-data/config/setup.example.kdl
```

The bundled example config uses `setup_mode="builder-compatible"` to produce
the older post-join active-campaign baseline used by maint/oracle sweeps.

Inspect a directory:

```bash
cd rust
cargo run -q -p ec-cli -- core-report /tmp/ec-game
```

Launch original `ECGAME` locally in DOSBox-X:

```bash
tools/run_ecgame.sh /path/to/game_dir
```

## Documentation

Start here:

- [docs/approach.md](docs/approach.md)
- [docs/next-session.md](docs/next-session.md)
- [docs/rust-architecture.md](docs/rust-architecture.md)

Key design docs:

- [docs/ec-combat-spec.md](docs/ec-combat-spec.md)
- [docs/ec-setup-spec.md](docs/ec-setup-spec.md)
- [docs/ec-movement-spec.md](docs/ec-movement-spec.md)
- [docs/config-architecture.md](docs/config-architecture.md)
- [docs/dosbox-workflow.md](docs/dosbox-workflow.md)

## Repository Layout

- `original/` original EC 1.5 materials used for preservation and validation
- `docs/` stable engineering and rules docs
- `RE_NOTES.md` chronological reverse-engineering notebook
- `rust/` Rust workspace (`ec-data`, `ec-cli`, tests)
- `tools/` oracle, DOSBox, and analysis helpers

## License

The new source code and tooling in this repository are licensed under the MIT
License. See [LICENSE](LICENSE).

The original Esterian Conquest DOS binaries, data files, manuals, logs, and
other preserved game materials remain original works of Bently C. Griffith and
their original rights holders. Their inclusion here is for preservation,
research, and compatibility work; they are not relicensed under MIT by this
repository.
