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

## Recent Changes (2026-03-18)

### ECGAME Runtime Error 201 — Fixed

- **Root cause:** `setup_classic_probe_game.py` wrote `PLANET.raw[0x03] = 0x00`
  for owned planets. ECGAME requires `0x87` (developed colony flag) to correctly
  interpret the planet record. Without it, planet detail view crashes with a BP
  range check error.
- **Fix:** setup script now passes `135` (`0x87`) as the second `planet-potential`
  argument. All 54 maint tests pass.

### DATABASE.DAT fog-of-war — owned planet population added

- `build_database_dat()` in `reports.rs` now unconditionally stamps each player's
  DATABASE.DAT entries for planets they own with full intel (name, owner,
  production, armies, batteries, discovery year).
- Previously this only happened through template-dependent `is_owned_unknown` and
  `planet_intel_events` paths, leaving owned planets as "UNKNOWN" on fresh games.

### ECGAME DATABASE.DAT overwrite behavior — discovered

- ECGAME regenerates `DATABASE.DAT` on player login, overwriting what
  `rust-maint` wrote.
- Only `is_orbit_record` scan marker entries (0x01-0x04) are confirmed as
  surviving ECGAME's login rewrite.
- **Open question:** what DATABASE.DAT field values does ECGAME require to
  preserve an entry across login? Understanding this is needed before the total
  planet database view will show player-owned planets correctly.

### Stardock display — open

- Stardock items in `PLANETS.DAT` are not shown in ECGAME's "Docked:" field.
- ECGAME may read docked ships from `DATABASE.DAT` or require additional state.
- The stardock data layout (u16 counts at 0x38-0x4B, u8 kinds at 0x4C-0x55) is
  confirmed correct from fixtures.

## Immediate Next Steps

1. **DATABASE.DAT ECGAME compatibility:** reverse-engineer which fields/markers
   ECGAME checks when deciding to display a DATABASE.DAT entry in the total
   planet database. The `is_orbit_record` markers (scan 0x01-0x04 with
   `raw[0x00]=0`) are the only confirmed pattern. Compare against an ECGAME
   session that runs the original `ECMAINT.EXE` to see what entries survive.
2. **Stardock display:** determine how ECGAME populates the "Docked:" field in
   planet detail view — does it read from DATABASE.DAT, PLANETS.DAT, or both?
3. Keep using the canonical docs as the source of truth while Rust code moves.
4. After meaningful `rust-maint` behavior changes, rerun:
   - `python3 tools/oracle_sweep.py --mode seeded`
   - `python3 tools/rust_maint_sweep.py --turns 3`
   - `cargo test -q`
5. Keep `next-session.md` short; move bulky historical detail to
   `archive/` instead of re-growing this file
