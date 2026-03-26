# Next Session

Keep this file short. Historical detail belongs in
[archive/next-session-archive.md](archive/next-session-archive.md), not here.

## Current State

- Public gameplay work is centered on `ec-game` and `ec-sysop`.
- `ec-game` is broadly feature-complete and the player TUI is in good shape.
  The main explicit remaining TUI placeholder is planet database filtering
  (`Range`, `Empire`, and `Max Production`).
- SQLite is the live runtime store and the runtime/storage architecture is now
  effectively production-complete for normal gameplay use.
- Snapshot families use normalized per-family tables rather than the old
  byte-offset `*_record_fields` tables.
- Runtime/gameplay code no longer dereferences classic record offsets directly
  under `ec-engine`, and shared `ec-data` runtime helpers are now using record
  accessors instead of open-coded `.raw[...]` reads.
- Runtime-only state such as reports, mail, intel, scorch orders, and theme
  preferences is already stored relationally.
- The intended runtime end state is still a **semantically normalized
  relational SQLite game state**.
- Snapshot storage no longer persists whole-record residue or grouped opaque
  tail slices.
- Remaining storage debt is now mostly semantic naming polish, not runtime DB
  plumbing.
- Latest broad baselines before new work:
  - `cargo test -q`
  - `cargo test -q -p ec-game`

## Current Goal

- Keep the Rust player client stable and finish the small remaining UI/admin
  polish tasks.
- Keep classic import/export and oracle tooling as compatibility backstops, not
  the primary day-to-day development model.
- Only deepen semantic field naming when it materially helps gameplay,
  tooling, or compatibility work.

## Biggest Blockers

- There is no major runtime-storage blocker left.
- The main remaining engineering work is incremental polish:
  - planet database filtering in `ec-game`
  - `ec-sysop` and surrounding admin workflow polish
  - semantic naming cleanup only where it pays for itself
- New gameplay features should not deepen the offset-shaped storage path.
- Remaining TUI work is now minor cleanup, not the primary blocker.

## Immediate Next Steps

1. Finish planet database filtering (`Range`, `Empire`, `Max Production`) in
   `ec-game`.
2. Keep `ec-sysop` moving toward the same level of completeness and polish as
   the player TUI.
3. Preserve the exact roundtrip storage tests and source-policy guardrails so
   runtime code does not drift back toward raw-offset dependence.
4. Only rename/decompose remaining classic-derived control fields when the
   semantics are clear and actually useful.
5. Keep this file concise and current instead of turning it back into a running
   notebook.
