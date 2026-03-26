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
- Snapshot storage no longer persists whole-record residue or grouped opaque
  tail slices.
- The remaining compatibility debt is now mainly semantic:
  some classic-derived fields, especially in `CONQUEST.DAT`, are stored as
  explicit offset-shaped raw columns because their gameplay meaning is still
  only partially understood.
- Latest broad baselines before new work:
  - `cargo test -q`
  - `cargo test -q -p ec-game`

## Current Goal

- Keep the Rust player client stable while shifting primary effort away from
  broad TUI implementation and toward runtime storage cleanup.
- Keep pushing the storage model toward smaller semantic fields and fewer
  offset-shaped raw fields, without regressing exact classic import/export.
- Keep classic import/export and oracle tooling as compatibility backstops, not
  the primary day-to-day development model.

## Biggest Blockers

- The main architectural blocker is now mostly about semantics, not runtime
  plumbing:
  - runtime/gameplay layers are off direct raw offsets
  - snapshot storage is normalized and exact-roundtrip safe
  - the remaining unresolved storage seam is that some classic control/header
    fields are still stored by explicit offsets rather than fully named
    semantics
- New gameplay features should not deepen the offset-shaped storage path.
- Remaining TUI work is now minor cleanup, not the primary blocker.

## Immediate Next Steps

1. Keep converting offset-shaped classic fields, especially in
   `CONQUEST.DAT`, into named semantic storage only when the decoded meaning is
   actually useful to gameplay or tooling.
2. Keep `CoreGameData` as the in-memory boundary for now, but continue pushing
   unknown byte semantics behind typed record accessors rather than exposing
   `.raw[...]` to runtime callers.
3. Keep the exact roundtrip storage tests green so the classic export contract
   stays explicit while semantic storage gradually expands.
4. Treat planet database filtering as the only notable remaining TUI cleanup,
   not as a broad unfinished command-surface category.
5. Keep this file concise and current instead of turning it back into a running
   notebook.
