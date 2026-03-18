# Next Session

Use this as the restart brief. Historical detail belongs in
[next-session-archive.md](archive/next-session-archive.md),
not here.

## Current State

`rust-maint` is stable as a full-year maintenance engine and remains green
against the current oracle suite.

Current high-confidence status:

- the canonical turn-order, timing, combat, movement, and economy docs in this
  directory are now current enough to drive Rust implementation directly
- no known turn-cycle or timing oracle questions currently block Rust clone
  development
- the remaining timing/report uncertainties are low-value static/helper trivia,
  not engine-behavior blockers
- setup generation covers the documented player tiers and continues to produce
  classic-compatible directories accepted by the original toolchain

Recent validation baseline:

- `python3 tools/oracle_sweep.py --mode seeded`
  - `12/12` zero-diff classic `ECMAINT` oracle passes across
    `4/9/16/25` players and seeds `1515/2025/4242`
- `python3 tools/rust_maint_sweep.py --turns 3`
  - `8/8` passes across `4/9/16/25` players and seeds `1515/2025`
- `cargo test -q`
  - workspace green at the last recorded sweep

## Canonical Docs

Use these first when changing engine behavior:

- [ec-turn-cycle-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-turn-cycle-spec.md)
- [rust-turn-cycle-implementation.md](/home/mag/dev/esterian_conquest/docs/dev/rust-turn-cycle-implementation.md)
- [ec-combat-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-combat-spec.md)
- [ec-timing-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-timing-spec.md)
- [economics.md](/home/mag/dev/esterian_conquest/docs/dev/economics.md)
- [ec-movement-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-movement-spec.md)
- [ec-setup-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-setup-spec.md)

For repo structure and workflow, also keep
[approach.md](/home/mag/dev/esterian_conquest/docs/dev/approach.md) and
[rust-architecture.md](/home/mag/dev/esterian_conquest/docs/dev/rust-architecture.md)
close at hand.

## Current Goal

The next major implementation phase is the Rust `ECGAME` replacement on top of
the new Rust-native runtime state, while keeping `rust-maint` honest through
repeated oracle validation and classic `.DAT` compatibility checks.

Practical posture:

- treat the current spec docs as the authority for rules/ordering
- only reopen rule recovery when a classic oracle diff, manual reading, or
  reproducible probe shows the Rust rule should move
- prefer implementation and regression coverage over more deep RE when the
  remaining questions are non-blocking

## Real Blockers

No known oracle/spec blockers remain in:

- turn ordering
- weekly timing / `Stardate` assignment
- canonical Rust combat mechanics

The main risks are implementation drift and regression, not missing core rules.

## Immediate Next Steps

1. keep using the canonical docs above as the source of truth while Rust code
   moves
2. after meaningful `rust-maint` behavior changes, rerun:
   - `python3 tools/oracle_sweep.py --mode seeded`
   - `python3 tools/rust_maint_sweep.py --turns 3`
   - `cargo test -q`
3. keep `next-session.md` short; move bulky historical detail to
   `archive/` instead of re-growing this file
