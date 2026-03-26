# Next Session

Keep this file short. Historical detail belongs in
[archive/next-session-archive.md](archive/next-session-archive.md), not here.

## Current State

- Public gameplay work is centered on `ec-game` and `ec-sysop`.
- `ec-game` is broadly feature-complete and the player TUI is in good shape.
  The main explicit remaining TUI placeholder is planet database filtering
  (`Range`, `Empire`, and `Max Production`).
- SQLite is the live runtime store, but the current snapshot format is still
  partly transitional:
  - classic-backed game data is persisted through byte-oriented snapshot record
    tables
  - runtime-only state such as reports, mail, intel, scorch orders, and theme
    preferences is already stored relationally
- The intended runtime end state is still a **semantically normalized
  relational SQLite game state**.
- The byte-oriented snapshot record tables are compatibility debt, not the
  desired long-term gameplay schema.
- Latest broad baselines before new work:
  - `cargo test -q`
  - `cargo test -q -p ec-game`

## Current Goal

- Keep the Rust player client stable while shifting primary effort away from
  broad TUI implementation and toward runtime storage cleanup.
- Move authoritative gameplay snapshot storage toward normalized relational
  SQLite instead of extending raw record-byte storage by default.
- Keep classic import/export and oracle tooling as compatibility backstops, not
  the primary day-to-day development model.

## Biggest Blockers

- The main architectural blocker is still the mixed runtime storage model:
  - runtime-only state is properly relational
  - classic-derived snapshot records are still stored byte-by-byte through the
    live runtime DB
- New gameplay features should not casually deepen the transitional storage
  model.
- Remaining TUI work is now minor cleanup, not the primary blocker.

## Immediate Next Steps

1. Replace byte-oriented snapshot persistence behind `CampaignStore` with
   normalized relational SQLite tables while keeping `CoreGameData` as the
   in-memory boundary for the first pass.
2. Keep new runtime-only gameplay state relational by default and avoid adding
   more features to the transitional byte-table path.
3. Treat planet database filtering as the only notable remaining TUI cleanup,
   not as a broad unfinished command-surface category.
4. Use oracle work only when a concrete gameplay or compatibility blocker
   requires it.
5. Keep this file concise and current instead of turning it back into a running
   notebook.
