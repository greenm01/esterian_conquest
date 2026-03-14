# Next Session

Use this as the restart brief. Historical detail belongs in
[next-session-archive.md](/home/mag/dev/esterian_conquest/docs/next-session-archive.md),
not here.

## Current State

`rust-maint` is now end-to-end capable in a conservative, manual-faithful way.

It can currently:

- create new classic-compatible games across the documented `4 / 9 / 16 / 25`
  player tiers
- run repeated Rust maintenance turns over full campaign state
- handle movement, economy, scouting, contact reporting, diplomacy,
  deterministic combat, conquest, civil disorder, fleet defection, and
  conservative emperor recognition
- regenerate classic `DATABASE.DAT` and `RESULTS.DAT`
- write a first-pass routed `MESSAGES.DAT` stream from recipient-scoped maint
  events
- preserve existing classic player-mail `MESSAGES.DAT` payloads during
  `rust-maint` when no routed maint messages are emitted
- keep producing directories the original `ECMAINT` accepts

Recent validation:

- `python3 tools/oracle_sweep.py --mode seeded`
  - current result: `12/12` zero-diff `ECMAINT` oracle passes across
    `4/9/16/25` players and seeds `1515/2025/4242`
- `python3 tools/rust_maint_sweep.py --turns 3`
  - current result: `8/8` passes across `4/9/16/25` players and seeds
    `1515/2025`
- `cargo test -q`
  - current workspace status: green

## Current Goal

Primary goal:

- keep `rust-maint` honest as a full-game engine by continuing repeated oracle
  validation against classic `.DAT` output
- refine only where stronger manual evidence or original-binary evidence shows
  the Rust rule should move
- shift the next major implementation phase toward cloning `ECGAME` in Rust

## What Is Settled

- manuals are the semantic authority
- original DOS binaries are the compatibility oracle
- `.DAT` remains the compliance boundary
- hidden or stochastic original behavior may be reimplemented canonically if
  the result remains faithful to the manuals and stays classic-compatible
- deterministic Rust combat is the chosen canonical replacement for opaque
  original combat RNG
- `ECGAME` local DOSBox launch is now documented and working with the corrected
  local-console `CHAIN.TXT` settings in
  [`docs/dosbox-workflow.md`](/home/mag/dev/esterian_conquest/docs/dosbox-workflow.md)

## Biggest Remaining Engine Questions

- emperor-recognition details may still need refinement if stronger classic
  evidence appears
- fleet-defection cadence is currently conservative and deterministic, not
  proven byte-for-byte original behavior
- report wording and visibility can still be tightened when new `ECGAME` or
  manual evidence appears
- exact classic `MESSAGES.DAT` mail/report format and routing semantics are
  still only partially recovered; current Rust behavior preserves classic mail
  but does not yet decode or reproduce it faithfully

These are refinement tasks, not blockers for calling `rust-maint` a usable
full-game engine.

## Next Phase: Rust ECGAME

The next major phase should be cloning `ECGAME` in Rust while keeping the
existing `.DAT` compatibility boundary intact.

Initial scope:

- replicate the player-facing command flow and reports, not just the maint
  backend
- use the existing Rust maintenance/report pipeline instead of recreating game
  rules in a second place
- preserve classic terminology, menu structure, and campaign feel where the
  manuals or live `ECGAME` behavior are clear
- do not invent a surrender UI action; the manuals describe surrender as a
  campaign outcome, and live `ECGAME` evidence shows no General Command
  surrender option

First concrete work:

- document the `ECGAME` command/menu surface we want to clone first
- identify which current `ec-cli` report and inspection surfaces already cover
  those needs
- start a Rust `ECGAME` phase around:
  - status / reports / database viewing
  - diplomacy commands
  - order entry and review
  - classic player workflow around the existing Rust engine

## Immediate Next Steps

1. Keep running periodic seeded multi-turn `rust-maint` sweeps to guard against
   regressions while the UI/client work begins.
2. Write a focused Rust `ECGAME` phase plan:
   - command center
   - reports and intel views
   - diplomacy screens
   - order-entry workflow
3. Use the now-working DOSBox `ECGAME` harness to capture only the player-side
   screens and behaviors needed for the first Rust clone pass.
4. Keep SQLite and turn-limit policy deferred. They are approved future
   architecture, not the current milestone.
