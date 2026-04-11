# Next Session

Keep this file short. Historical detail belongs in
[archive/next-session-archive.md](archive/next-session-archive.md), not here.

## Current State

- Public gameplay is now centered on `nc-game`, `nc-door`, and `nc-sysop`.
- The old SSH/Nostr hosted path has been cut out of the active gameplay and
  sysop surfaces.
- `nc-sysop` now exposes only `new-game`, `maint`, and `settings`.
- `nc-sysop new-game` no longer seeds hosted seats in `ncgame.db`.
- `nc-game` no longer accepts hosted-only launch flags such as
  `--session-token` or `--hosted-invite-code`.
- `nc-dash` is still the native dashboard client, but not yet the hosted lobby.
- The BBS door client is verified on Mystic and ENiGMA½.
- Latest local baselines after the hosted-path cut:
  - `cargo test -q -p nc-session`
  - `cargo test -q -p nc-game`
  - `cargo test -q -p nc-sysop`

## Current Goal

- Stabilize the localhost and BBS release surfaces for public beta use.
- Fix real playtest bugs and workflow rough edges quickly.
- Keep docs aligned with the local/BBS product story.
- Treat future Nostr work as a separate `nc-daemon` / `nc-dash` track.

## Biggest Blockers

- No known major engine/storage blocker remains.
- The main remaining risk is field bugs found by real BBS and localhost players.
- `nc-dash` still needs its future lobby/hosted architecture if hosted play is
  revisited.
- The BBS door renderer still repaints full frames instead of using the
  retained-frame diffing already in local `nc-game`.

## Immediate Next Steps

1. Keep running real BBS and localhost playtests and tighten the rough edges.
2. Regenerate manuals/PDFs whenever the public command surface changes.
3. Keep runtime/storage code pointed at SQLite and away from raw offset-style
   regressions.
4. If hosted work resumes, do it as the new `nc-daemon` / `nc-dash` path, not
   by reviving the retired SSH bridge stack.
