# Next Session

Use this as the restart brief. Historical detail belongs in
[next-session-archive.md](archive/next-session-archive.md), not here.

## Current State

- Architecture is stable:
  - `ec-data` = runtime/store/model
  - `ec-engine` = gameplay/rules API
  - `ec-compat` = classic `.DAT` import/export/oracle bridge
  - `ec-classic` = low-level classic record/codecs
  - public binaries are converging on `ec-game` + `ec-sysop`
  - `ec-cli` remains the internal developer/oracle surface
- Phase 1 engine-boundary correction is now in place:
  - maintenance execution lives in `ec-engine/src/maint/`
  - shared maintenance event/result payloads live in
    `ec-data::maintenance_types`
  - `ec-data` no longer exports maintenance execution entrypoints
- Phase 2 boundary correction is now in place:
  - movement/pathfinding rule code lives in `ec-engine/src/navigation/`
  - raw fleet motion scratch-byte helpers live in
    `ec-data::fleet_motion_state`
- Phase 3 boundary correction is now in place:
  - setup/map-generation rule code lives in `ec-engine/src/setup/`
  - shared setup config parsing and baseline state builders remain in
    `ec-data`
- SQLite is the runtime source of truth for engine and TUI.
- Classic `.DAT` files are now an explicit compatibility edge, not live runtime
  state.
- Latest full baseline passed:
  - `cargo test -q`
- The heavy oracle / reverse-engineering phase is now considered closed for
  normal development:
  - movement, maintenance timing, combat placement, and economy semantics are
    documented to the level currently needed by the Rust engine
  - the original DOS binaries remain the compatibility oracle and provenance
    source, not an active day-to-day research queue

## Current Goal

Advance the Rust-first game and client without reopening oracle work unless a
concrete compatibility or gameplay blocker appears.

The core recovered rule areas are considered settled enough for ongoing
implementation:

- yearly maintenance ordering and report placement
- movement and mission arrival semantics
- seeded Rust combat inside the recovered classic timing framework
- canonical Rust economy rules, including the current starbase growth policy
- classic `.DAT` import/export as an explicit compatibility boundary

## Biggest Blockers

- Player-facing Rust work:
  - continue the TUI and gameplay-facing command surfaces on top of the stable
    SQLite/runtime model
- Compatibility hygiene:
  - keep classic import/export and oracle sweeps available as regression tools
  - reopen deep RE only for a concrete diff, crash, or gameplay mismatch that
    materially blocks the Rust engine
- Ongoing implementation polish:
  - weekly report timing and dated-report details should still be improved when
    they materially affect hybrid play or player-visible correctness

## Working Assumption

The original manuals and binaries remain the reference for intended rules and
compatibility, but the Rust project no longer treats open-ended oracle digging
as a standing priority.

Use the oracle stack for:

- import/export validation
- preserved-fixture replay checks
- targeted regression confirmation when a real mismatch appears

Do **not** reopen Ghidra/DOSBox/oracle investigation just to chase hidden
bytes, low-signal numeric quirks, or historical trivia that does not materially
affect gameplay, reports, or file safety.

## Immediate Next Steps

1. Keep building the Rust-first player/TUI flow and gameplay surfaces.
2. Use oracle tooling only as a compatibility/regression backstop.
3. Reopen deep RE only for a concrete blocker.
4. Keep `docs/dev/archive/RE_NOTES.md` archival; do not turn
   `docs/dev/next-session.md` back into an oracle notebook.

## Structural Note

The three major gameplay subsystems now follow the intended split:

- `ec-engine` owns maintenance, movement/pathfinding, and setup/map-generation
  rule execution
- `ec-data` keeps runtime/store/model state plus shared config, builder, and
  raw record-layout helpers needed by those engine systems
