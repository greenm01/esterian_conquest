# Next Session

Keep this file short. Historical detail belongs in
[archive/next-session-archive.md](archive/next-session-archive.md), not here.

## Current State

- Public gameplay work is centered on `ec-connect`, `ec-game`, and
  `ec-sysop`.
- `ec-game` is broadly feature-complete and the player TUI is in good shape.
- `ec-sysop` is also in good enough shape for normal campaign operation.
- The total planet database now supports both `F` filters and `S` sorting.
- SQLite is the live runtime store and the runtime/storage architecture is now
  effectively production-complete for normal gameplay use.
- Snapshot families use normalized per-family tables rather than the old
  byte-offset `*_record_fields` tables.
- Runtime/gameplay code no longer dereferences classic record offsets directly
  under `ec-engine`, and shared `ec-data` runtime helpers are now using record
  accessors instead of open-coded `.raw[...]` reads.
- Runtime-only state such as reports, mail, intel, scorch orders, and theme
  preferences is already stored relationally.
- The project is now effectively in a **beta / playtest** phase:
  - core player/connect/sysop workflows exist
  - the main remaining unknowns are real-world usability issues and bugs found
    during campaign play
- The Rust BBS door client is now verified on both Mystic and ENiGMA½.
- For BBS play, the stable door control contract is `HJKL` movement, `^U` /
  `^D` paging, and `Q` / `Esc` for back/quit.
- Latest broad baselines before new work:
  - `cargo test -q`
  - `cargo test -q -p ec-game`
  - `cargo test -q -p ec-sysop`

## Current Goal

- Keep the Rust player and sysop surfaces stable during real playtesting.
- Collect player/sysop feedback and fix reported bugs, rough edges, and
  workflow confusion quickly.
- Keep classic import/export and oracle tooling as compatibility backstops, not
  the primary day-to-day development model.

## Biggest Blockers

- There is no major runtime-storage blocker left.
- There is no known major player-TUI feature gap left.
- The main remaining risk is unknown bugs or confusing workflows that only show
  up under real player/sysop use.
- New gameplay features should not deepen the offset-shaped storage path.

## Immediate Next Steps

1. Run real player and sysop playtests and capture friction points, crashes,
   unclear prompts, and campaign-operation pain points.
2. Fix reported bugs and UX issues in small, well-tested increments.
3. Preserve the storage roundtrip tests and source-policy guardrails so runtime
   code does not drift back toward raw-offset dependence.
4. Revisit a future universal `Ctrl-/` bordered help popup with visible padding
   so screen-local key discoverability can move out of crowded command rails.
5. Only do deeper semantic cleanup when it materially helps a real gameplay,
   playtest, or compatibility issue.
