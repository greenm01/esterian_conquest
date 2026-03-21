# Reverse Engineering and Compatibility

This directory holds the deeper oracle, provenance, and binary-recovery
material that used to spill into the main README.

The front page now treats this repository primarily as the Rust `v1.6`
continuation of Esterian Conquest. This folder is where the detailed
"how we proved it" story lives.

## Current Posture

- the Rust engine, maint pipeline, and classic export layer are now far enough
  along that active work should bias toward `maint-rust` polish and the Rust
  player client/TUI
- original DOS binaries remain the compatibility oracle, not the product front
  end
- deep RE should reopen only when a concrete oracle diff, crash, or gameplay
  mismatch requires it

Recent proof baseline:

- `python3 -u tools/oracle_sweep.py --mode seeded`
  - current result: `12/12` passes
- `python3 tools/rust_maint_sweep.py --turns 3`
  - current result: `8/8` passes
- `cargo test -q -p ec-cli --test storage`
  - current result: `15 passed, 6 ignored`

## Where To Read

- [docs/dev/approach.md](/home/niltempus/dev/esterian_conquest/docs/dev/approach.md)
  - project compatibility policy, milestone ladder, and deliberate divergence
    rules
- [docs/dev/next-session.md](/home/niltempus/dev/esterian_conquest/docs/dev/next-session.md)
  - current restart brief and remaining compatibility work
- [docs/dev/archive/RE_NOTES.md](/home/niltempus/dev/esterian_conquest/docs/dev/archive/RE_NOTES.md)
  - chronological archival notebook
- [EC_UNLOCKED/README.md](/home/niltempus/dev/esterian_conquest/EC_UNLOCKED/README.md)
  - decrypted runnable binaries used for static and dynamic analysis
- [docs/dev/dosbox-workflow.md](/home/niltempus/dev/esterian_conquest/docs/dev/dosbox-workflow.md)
  - oracle-running and dump-capture workflow
- [docs/dev/ghidra-workflow.md](/home/niltempus/dev/esterian_conquest/docs/dev/ghidra-workflow.md)
  - headless/static RE workflow

## Core Rules

- the manuals in `original/v1.5/*.DOC` are the player-facing gameplay source
- the original binaries are the compatibility oracle
- classic `.DAT` files are the interchange boundary
- documented original logic bugs that do not protect file safety are recorded,
  but not intentionally reproduced in Rust

## Known Deliberate Divergences

- Rust does not reproduce the original lone-active `ScoutSolarSystem` abort
  bug in `ECMAINT`
- Rust does not reproduce the legacy rogue-viewer foreign-scout refresh quirk
- Rust keeps deterministic, explicit behavior where the original implementation
  was hidden, stochastic, or not worth cloning literally
