# Next Session

Keep this file short. Historical detail belongs in
[archive/next-session-archive.md](archive/next-session-archive.md), not here.

## Completed This Session

**nc-helm starmap UX overhaul** — zoom removed (unused, no gameplay value);
center-follow viewport replaced with 2-sector dead-zone margin scrolling;
middle-click drag-to-pan ("grab the map") with middle-click re-center on
crosshair; crosshair overlay hidden when outside viewport. All nc-helm tests
green.

## Current Goal

- Stabilize the localhost and BBS release surfaces for public beta use.
- Fix real playtest bugs and workflow rough edges quickly.
- Keep docs aligned with the local/BBS product story.
- Treat future Nostr work as a separate `nc-host` / `nc-helm` track.

## Biggest Blockers

- No known major engine/storage blocker remains.
- The main remaining risk is field bugs found by real BBS and localhost players.
- The hosted `nc-host` / `nc-helm` track now exists locally, but it is still a
  dev-only path and not the public shipped product story.
- The biggest remaining hosted client gap is finishing the last unsupported
  hosted dashboard actions and broadening the staged hosted draft flow beyond
  the current fleet-order and planet-build surfaces.
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
6. Keep the shared local/hosted dashboard launch path stable while real users
   exercise the hosted dashboard.
7. Finish the remaining unsupported hosted dashboard actions or explicitly
   scope them out of the hosted flow.
8. If modal drag pacing still feels rough in `nc-helm`, evaluate a future
   GPU/vsynced native renderer path as follow-up work.
9. After `nc-helm` is stable on the shared ratatui render contract, replace
   the temporary `crossterm` event types with a small local input model.
