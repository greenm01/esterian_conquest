# Next Session

Keep this file short. Historical detail belongs in
[archive/next-session-archive.md](archive/next-session-archive.md), not here.

## Current State

- Public gameplay work is centered on `ec-connect`, `ec-game`, and
  `ec-sysop`.
- Hosted Rust campaigns are now DB-only: `ec-sysop new-game` creates only
  `ecgame.db`.
- `ec-sysop` now owns SQLite-native `new-game`, `settings`, `maint-all`, and
  host game-registry commands directly instead of delegating to `ec-cli`.
- `ec-game` and `ec-sysop` normal dependency graphs no longer pull
  `ec-compat` / `ec-classic`.
- `ec-gate` now reads game names and seat/session metadata from `ecgame.db`
  and issues per-seat session leases to block duplicate logins.
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
- The SSH/local `ec-game` renderer now diffs retained frames instead of
  clearing the whole terminal every keypress; the BBS door path still uses
  full-frame repaint and should get the same treatment later.
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

## Recent Fixes

- **Hosted Rust runtime decoupled from classic packaging** (fixed):
  `ec-sysop new-game` now writes only `ecgame.db`; no `.DAT` files, classic
  executables/docs, `config.kdl`, or `themes/` are produced in hosted game
  directories.
- **Multi-game host operations moved into `ec-sysop`** (fixed):
  per-game settings now live in SQLite, `maint-all` sweeps registered games
  from the gate config, and host `games add/remove/list` plus `status` now
  manage and inspect the daemon-facing game registry.
- **Hosted first-join routing and seat-claim guardrails tightened** (fixed):
  hosted players now stay on the dedicated empire-naming flow instead of
  falling through generic first-time screens, and one hosted identity can no
  longer claim multiple seats in the same game.

## Biggest Blockers

- There is no major runtime-storage blocker left.
- There is no known major player-TUI feature gap left.
- The main remaining risk is unknown bugs or confusing workflows that only show
  up under real player/sysop use.
- `ec-connect`'s post-`ec-game` first-key fix is currently Unix-only; Windows
  still needs a bridge-side stdin shutdown path so the first returned keypress
  cannot be stolen after the SSH session exits.
- `ec-connect` is still single-relay; multi-relay join/handshake redundancy is
  a future resilience improvement, not part of the current player fix stream.
- `ec-connect` cache rows are still not modeled for the legitimate edge case of
  one local wallet keeping multiple seats in the same hosted game under
  different identities; that is a separate follow-up from current management
  fixes.
- The BBS door renderer still repaints full frames and may show the same flash
  that was just fixed for SSH/local play.
- New gameplay features should not deepen the offset-shaped storage path.

## Immediate Next Steps

1. Run VPS and live multi-game playtests against the new DB-only hosted layout.
2. Fix reported bugs and UX issues in small, well-tested increments.
3. Preserve the storage roundtrip tests and source-policy guardrails so runtime
   code does not drift back toward raw-offset dependence.
4. Revisit a future universal `Ctrl-/` bordered help popup with visible padding
   so screen-local key discoverability can move out of crowded command rails.
5. Add the Windows half of the `ec-connect` bridge stdin shutdown fix so
   post-game return behavior matches Linux/macOS and the first keypress works
   immediately after leaving `ec-game`.
6. Revisit multi-relay support for `ec-connect` join/handshake flows if relay
   reliability remains a recurring playtest problem.
7. Only do deeper semantic cleanup when it materially helps a real gameplay,
   playtest, or compatibility issue.
