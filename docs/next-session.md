# Next Session

Use this as the restart brief. Historical detail belongs in
[next-session-archive.md](docs/next-session-archive.md),
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
- generate default `sysop new-game` directories as joinable `ECGAME` starts
  again:
  - inactive player slots
  - `Not Named Yet` homeworld seeds
  - pre-join fleet blocks at seeded homeworld coords
- keep the older post-join active campaign baseline available through
  `setup_mode="builder-compatible"` for maint/oracle sweeps and test fixtures

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
  [`docs/dosbox-workflow.md`](docs/dosbox-workflow.md)
- planet economy now has an explicit canonical Rust rule where the original
  replay oracle is still awkward to probe directly:
  - empire-wide tax sets yearly revenue on every owned planet
  - lower tax accelerates current-production growth toward potential
  - taxes above `65%` can now directly reduce present production
  - starbases boost growth and build capacity
  - civil-disorder baselines are left alone so preserved maint fixtures stay
    stable
- the canonical economy rule is now documented in
  [economics.md](docs/economics.md)
- builder-generated starts now encode the intended opening economy directly:
  - homeworld current production starts at `100`
  - default empire tax starts at `50%`
  - canonical initialized homeworlds start with `10` armies and `4` batteries
- a focused original-`ECMAINT` probe now shows that letting a ship build complete
  into a full stardock is unsafe:
  - the build slot clears
  - no `ERRORS.TXT` is emitted
  - the target planet's stardock bytes are corrupted
  - Rust now keeps blocked ship/starbase builds queued unchanged until a
    stardock slot opens, while armies and batteries still complete normally
  - keep the Rust client-side stardock-capacity guard in place
- a focused original-byte-limit probe now shows:
  - planet armies at `255` stay at `255` and still consume a completing army build
  - planet batteries at `255` stay at `255` and still consume a completing battery build
  - a simple scout-fleet merge probe is not a clean overflow oracle because
    classic merge processing appears to drop merged-away scouts even below `255`
  - keep the Rust planet unload cap guard in place for now
  - the exact original `ECGAME` load/unload UI behavior above `255` is still
    worth a stronger screen-aware probe later
  - Rust now diverges intentionally on the planet-side byte caps:
    - army/battery builds that would overflow stay queued
    - unload to a full planet is rejected cleanly in the client and engine

## Biggest Remaining Engine Questions

- player-facing production semantics are not fully decoded yet:
  - original `ECGAME` exposes `Present Production`, `Potential Production`,
    `Total Available Points`, and empire/planet production rankings
  - Rust still has raw/RE-facing economic field names like `factories` for
    underlying Borland Pascal `Real` storage
  - next engine/UI alignment work should decode and expose the original
    production semantics instead of leaking raw field names into client screens
- `PLANETS.DAT raw[0x0E]` is not a settled planet-tax field:
  - mixed-tax Rust probes show it being overwritten during the existing
    autopilot/rogue AI path
  - do not treat `planet_tax_rate_raw()` as a stable player-facing semantic
    field after maintenance until that byte is fully decoded
- fleet numbering now has an important split to preserve:
  - preserved `ECGAME` logs strongly suggest the displayed `Nth Fleet` number is
    per-empire
  - the shipped active `original/v1.5/FLEETS.DAT` also shows per-owner local
    slots alongside globally unique structural fleet IDs, so those two fields
    should stay distinct in the Rust model
  - the current recovered structural fleet-chain model still treats
    `FLEETS.DAT record[0x05]` as a separate global linkage key
  - keep player-facing fleet numbering and structural fleet linkage distinct
    until deeper oracle evidence proves they are the same field
- emperor-recognition details may still need refinement if stronger classic
  evidence appears
- fleet-defection cadence is currently conservative and deterministic, not
  proven byte-for-byte original behavior
- report wording and visibility can still be tightened when new `ECGAME` or
  manual evidence appears
- exact classic `MESSAGES.DAT` mail/report format and routing semantics are
  still only partially recovered; current Rust behavior preserves classic mail
  but does not yet decode or reproduce it faithfully
- live `ECGAME` confirmation is still needed that the restored default
  joinable setup now triggers the full first-join naming/onboarding flow

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
- for the Rust client, present official maintenance/results reports before
  player-to-player mail so reports reveal outcomes before social commentary can
  spoil them

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

1. Verify the restored default `sysop new-game` path in live `ECGAME`:
   - join as player 1
   - confirm homeworld naming prompt appears
   - confirm player 2 can also join cleanly afterward
2. Keep running periodic seeded multi-turn `rust-maint` sweeps to guard against
   regressions while the UI/client work begins.
3. Write a focused Rust `ECGAME` phase plan:
   - command center
   - reports and intel views
   - diplomacy screens
   - order-entry workflow
   - defer real `X`/expert-mode behavior until the remaining command/menu
     surfaces are finished; implement it as a final menu-verbosity pass rather
     than a premature partial toggle
4. Keep tightening original production semantics for player-facing screens:
   - empire profile / rankings / planet info should use classic terms like
     `Present Production`, `Potential Production`, and `Total Available Points`
   - do not expose raw internal names like `factories` in the client UI
   - if stronger oracle evidence appears, refine the canonical Rust growth
     formula rather than reintroducing placeholder arithmetic
   - decode or rename the overloaded per-planet `raw[0x0E]` byte before using
     it for more player-facing economy output
5. Use the now-working DOSBox `ECGAME` harness to capture only the player-side
   screens and behaviors needed for the first Rust clone pass.
6. Keep SQLite and turn-limit policy deferred. They are approved future
   architecture, not the current milestone.
