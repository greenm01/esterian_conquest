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
- `ncgame.db` has dropped the retired hosted/session tables entirely.
- `nc-gate` is no longer part of the active Rust workspace build.
- `nc-client` now exists as the shared hosted client core.
- `nc-dash` now has a partial hosted lobby/client path in the same binary.
- That hosted lobby path now covers public `30500` discovery, invite
  request/decision, invite claim, runtime-backed `30520` state refresh,
  turn submit/receipt, public `30516` notices, and encrypted `30517`
  sysop thread messages.
- `nc-dash` lobby now keeps a live hosted observer session instead of doing
  full reconnect/fetch cycles for catalog, notice, thread, and inbox updates.
- `30520 GameState` now uses typed hosted snapshot payloads instead of opaque
  JSON blobs on the Rust side.
- `nc-dash` hosted-game route now builds and renders a real `DashApp` from
  typed hosted snapshots instead of using the old separate mini summary view.
- `nc-host` now exists as the relay-native hosted server name and localhost dev lab target.
- `nc-host` now exposes `notices` and `threads` operator commands for the
  hosted lobby communication surfaces.
- The BBS door client is verified on Mystic and ENiGMA½.
- Latest local baselines after the hosted-path cut:
  - `cargo test -q -p nc-session`
  - `cargo test -q -p nc-game`
  - `cargo test -q -p nc-sysop`

## Current Goal

- Stabilize the localhost and BBS release surfaces for public beta use.
- Fix real playtest bugs and workflow rough edges quickly.
- Keep docs aligned with the local/BBS product story.
- Treat future Nostr work as a separate `nc-host` / `nc-dash` track.

## Biggest Blockers

- No known major engine/storage blocker remains.
- The main remaining risk is field bugs found by real BBS and localhost players.
- The hosted `nc-host` / `nc-dash` track now exists locally, but it is still a
  dev-only path and not the public shipped product story.
- The biggest remaining hosted client gap is replacing the current synthesized
  hosted launch adapter with a true shared local/hosted dashboard launch model
  and a first-class hosted order submission path.
- The BBS door renderer still repaints full frames instead of using the
  retained-frame diffing already in local `nc-game`.

## Immediate Next Steps

1. Keep running real BBS and localhost playtests and tighten the rough edges.
2. Regenerate manuals/PDFs whenever the public command surface changes.
3. Keep runtime/storage code pointed at SQLite and away from raw offset-style
   regressions.
4. Keep the runtime DB scoped to localhost/BBS play; future hosted work should
   use its own schema rather than reviving retired hosted tables in `ncgame.db`.
5. Keep the localhost `nc-host` lab reproducible with the user-service install
   script and dev docs, but keep the public docs centered on local/BBS play.
6. Replace the current synthesized hosted dashboard adapter with a shared
   `DashLaunchState`-style path for both local and hosted play.
7. Move hosted order editing/submission off the raw-text modal and onto a
   first-class hosted dashboard flow.
