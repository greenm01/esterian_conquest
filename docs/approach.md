# Preservation Approach

This repository is not trying to recover the original Pascal source code verbatim.

The goal is:

- preserve Esterian Conquest v1.5 as a working historical artifact
- reverse engineer its file formats and rules
- build a faithful modern reimplementation in Rust
- keep the original DOS binaries and data as the reference implementation

## Principles

1. Treat the DOS binaries as the spec

- `ECGAME.EXE` is the player-facing command UI
- `ECUTIL.EXE` is the sysop/configuration utility
- `ECMAINT.EXE` is the yearly maintenance and simulation engine

2. Prefer confirmed behavior over guessed structure

- only name fields after they are supported by diffs, screenshots, docs, or repeated observation
- keep unknown bytes raw until they are mapped with confidence

3. Separate stable docs from lab notes

- `RE_NOTES.md` is the chronological investigation notebook
- `docs/` holds stable, reusable engineering docs

4. Keep the architecture layered

- `ec-data`: binary formats and typed accessors
- `ec-cli`: std-only scripting and verification interface
- `ec-tui`: user-facing terminal UI

5. Use fixtures to lock in behavior

- original shipped state
- initialized state
- post-maintenance state
- targeted scenario snapshots for specific features

## What Counts As Success

Short term:

- decode the important on-disk formats
- reproduce `ECUTIL` behavior faithfully
- understand `ECMAINT` as a deterministic state transformer

Long term:

- reimplement the real turn engine in Rust
- build a usable player client and admin client
- support classic-compatible saves and reproducible results

## Current Strategy

Near-term effort should prioritize `ECMAINT`.

Why:

- `ECUTIL` is mostly configuration/state setup
- `ECGAME` is mainly command entry and presentation
- `ECMAINT` appears to be the core simulation engine:
  - movement
  - battles
  - build completion
  - AI / rogue empire behavior
  - database and report generation

That makes `ECMAINT` the highest-value target for recovering the actual rules of the game.
