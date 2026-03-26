# Next Session

Keep this file short. Historical detail belongs in
[archive/next-session-archive.md](archive/next-session-archive.md), not here.

## Current State

- Public gameplay work is centered on `ec-game` and `ec-sysop`.
- `ec-game` is broadly feature-complete and the player TUI is in good shape.
  The main explicit remaining TUI placeholder is planet database filtering
  (`Range`, `Empire`, and `Max Production`).
- SQLite is the live runtime store and snapshot families now use normalized
  per-family tables rather than the old byte-offset `*_record_fields` tables.
- Runtime/gameplay code no longer dereferences classic record offsets directly
  under `ec-engine`, and shared `ec-data` runtime helpers are now using record
  accessors instead of open-coded `.raw[...]` reads.
- Runtime-only state such as reports, mail, intel, scorch orders, and theme
  preferences is already stored relationally.
- The intended runtime end state is still a **semantically normalized
  relational SQLite game state**.
- The remaining compatibility debt is at the storage boundary:
  `snapshot_core` still persists record-wide `compat_raw_hex` residue for exact
  classic roundtrips, especially around large opaque `SETUP.DAT` and
  `CONQUEST.DAT` regions.
- Latest broad baselines before new work:
  - `cargo test -q`
  - `cargo test -q -p ec-game`

## Current Goal

- Keep the Rust player client stable while shifting primary effort away from
  broad TUI implementation and toward runtime storage cleanup.
- Finish the last storage transition so authoritative gameplay snapshots no
  longer depend on record-wide raw residue in SQLite.
- Keep classic import/export and oracle tooling as compatibility backstops, not
  the primary day-to-day development model.

## Biggest Blockers

- The main architectural blocker is now narrower but sharper:
  - the runtime and gameplay layers are mostly off raw offsets
  - the SQLite snapshot layer still preserves whole-record residue through
    `compat_raw_hex`
  - exact classic export still depends on those preserved bytes for unresolved
    `SETUP.DAT` / `CONQUEST.DAT` regions
- New gameplay features should not deepen the raw-residue storage path.
- Remaining TUI work is now minor cleanup, not the primary blocker.

## Immediate Next Steps

1. Replace `compat_raw_hex` in `snapshot_core` with explicit stored fields or
   narrowly-scoped opaque slices, starting with the currently unresolved
   `SETUP.DAT` and `CONQUEST.DAT` families.
2. Keep `CoreGameData` as the in-memory boundary for now, but continue pushing
   unknown byte semantics behind typed record accessors rather than exposing
   `.raw[...]` to runtime callers.
3. Add storage roundtrip tests that fail once record-wide residue disappears,
   so the classic export contract stays explicit during the final schema pass.
4. Treat planet database filtering as the only notable remaining TUI cleanup,
   not as a broad unfinished command-surface category.
5. Keep this file concise and current instead of turning it back into a running
   notebook.
