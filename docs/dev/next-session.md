# Next Session

Keep this file short. Historical detail belongs in
[archive/next-session-archive.md](archive/next-session-archive.md), not here.

## Current State

- Public gameplay work is centered on `ec-game` and `ec-sysop`.
- SQLite is the live runtime store, but the current snapshot format is still
  partly transitional:
  - classic-backed game data is persisted through byte-oriented snapshot record
    tables
  - runtime-only state such as reports, mail, intel, and theme preferences is
    already stored relationally
- The intended runtime end state is still a **semantically normalized
  relational SQLite game state**.
- The byte-oriented snapshot record tables are compatibility debt, not the
  desired long-term gameplay schema.
- Latest broad baseline before new work: `cargo test -q`

## Current Goal

- Keep improving the Rust-first player/TUI experience.
- Move new runtime-only gameplay state toward normalized relational SQLite
  storage instead of extending raw record-byte storage by default.
- Keep classic import/export and oracle tooling as compatibility backstops, not
  the primary day-to-day development model.

## Biggest Blockers

- More player-facing command surfaces still need to be implemented cleanly in
  the TUI.
- Runtime storage architecture is mixed:
  - some state is properly relational
  - classic-derived snapshot records are still stored byte-by-byte
- New gameplay features should not casually deepen the transitional storage
  model.

## Immediate Next Steps

1. Continue implementing missing player commands in `ec-game`.
2. Store new runtime-only gameplay state relationally in SQLite unless classic
   compatibility specifically requires raw record-byte storage.
3. Use oracle work only when a concrete gameplay or compatibility blocker
   requires it.
4. Keep this file concise and current instead of turning it back into a running
   notebook.
