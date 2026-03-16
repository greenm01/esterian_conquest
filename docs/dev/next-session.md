# Next Session

Use this as the restart brief. Historical detail belongs in
[next-session-archive.md](next-session-archive.md),
not here.

## Current State

`rust-maint` is now end-to-end capable in a conservative, manual-faithful way.

The maintenance engine is also now on much firmer authority footing:

- major player-authored inputs are validated in shared `ec-data`, not trusted
  from the client
- malformed player state is sanitized and reported during `maint-rust` instead
  of being silently executed
- `core-validate` now audits gameplay-invalid player input, not just structural
  linkage
- deterministic malformed-directory stress coverage now exists at both
  `ec-data` and `ec-cli` layers

Current grade:

- maintenance engine authority / invalid-input resistance: `A+`
- maintenance engine behavior against `ECPLAYER.DOC`: `A+`
- overall `rust-maint` status: `A+`

Local development baseline:

- Rust builds already use Cargo's normal multi-core job scheduling by default
- `sccache` is now the recommended local compile-speed dependency
- do not treat `mold` as a required repo dependency; keep it optional/local

Latest oracle signal against the remaining manual-adjacent fleet assumptions:

- confirmed in classic `ECMAINT`:
  - `Seek Home` dynamically retargets when the nearer refuge is lost
  - `Guard a Starbase` follows a moved base
  - invalid guard-starbase linkage aborts with `ERRORS.TXT`
  - patrol/contact reports include actionable hostile composition
  - battle-loss reports include observed enemy composition and enemy losses inflicted
  - owned-world `Salvage` succeeds from a live classic probe:
    - the fleet moves to the owned world
    - the fleet is removed on arrival
    - classic reports an estimated recovered production yield
  - salvage failure at non-owned targets aborts and seeks home
  - `Join another fleet` hot pursuit is now confirmed from a player-authored
    classic `ECGAME` + `ECMAINT` probe:
    - `ECGAME` stores the host fleet number in mission aux and snapshots the
      host's current coordinates
    - later `ECMAINT` turns refresh the joiner's target to the host's new live
      location
    - on arrival, the host absorbs the joining fleet
  - surviving retreat after fleet combat is now confirmed from a player-authored
    classic bombardment probe:
    - the surviving fleet aborted its mission
    - switched to a seek-home retreat
    - reported enemy composition, enemy losses inflicted, own losses, and the
      named retreat destination
- confirmed known classic defect:
  - empty-sector salvage reuses the wrong failure text
    (`Since we no longer own the world...`) even when no world exists there
- confirmed in live `ECGAME` login probing:
  - `fixtures/ecmaint-fleet-battle-pre/v1.5` is maint-valid but not a valid
    returning-player client fixture when the persisted handle does not match the
    caller/dropfile identity:
    - classic enters the first-time menu
  - changing only the persisted slot-2 handle from `FOO` to `SYSOP`
    (matching the generated `CHAIN.TXT` alias) is enough to flip classic into
    a matched pre-loaded-player path
  - that matched path is distinct from both the first-time menu and the normal
    established-player login:
    - intro pages
    - one-time empire rename prompt
    - status screen
    - report/message review
    - homeworld naming
    - then `MAIN MENU`
  - `ec-cli inspect-classic-login <dir> <caller_alias>` now reports the
    compatibility-layer classification Rust expects for each slot:
    `first-time-menu`, `matched-preloaded-first-login`, or
    `returning-player`
  - `ec-cli classic-login-prepare <dir> <player_record> <caller_alias>
    [empire_name]` now provides a narrow local-probe helper that aligns the
    persisted player handle with the caller alias without changing broader
    gameplay state

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
- support a documented local hybrid campaign loop:
  - Rust creates the campaign
  - classic `ECGAME` launches against the same working directory
  - `classic-login-prepare` can align a local caller alias with a persisted
    player handle for matched probes
  - `maint-rust` advances the same directory and reprojects classic files back
    into place

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
- shift the next major implementation phase toward cloning `ECGAME` in Rust on
  top of the new Rust-native SQLite campaign store

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
  [`docs/dosbox-workflow.md`](dosbox-workflow.md)
- planet economy now has an explicit canonical Rust rule where the original
  replay oracle is still awkward to probe directly:
  - empire-wide tax sets yearly revenue on every owned planet
  - lower tax accelerates current-production growth toward potential
  - taxes above `65%` can now directly reduce present production
  - starbases boost growth and build capacity
  - civil-disorder baselines are left alone so preserved maint fixtures stay
    stable
- the canonical economy rule is now documented in
  [economics.md](economics.md)
- builder-generated starts now encode the intended opening economy directly:
  - homeworld current production starts at `100`
  - default empire tax starts at `50%`
  - when a player joins a fresh slot, the claimed homeworld now starts with the
    opening spendable production implied by the manuals: `50` stored points at
    the default `50%` tax rate on `100` present production
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
- the newest oracle pass closed the remaining fleet/manual uncertainty:
  - `Seek Home`, `Guard Starbase`, `Join another fleet`, patrol contact intel,
    salvage success/failure semantics, and surviving retreat/abort reporting now
    all have direct classic evidence
- the combat spec now includes an explicit contact / hostility escalation
  matrix:
  - neutral deep-space transit is separate from neutral hostile local intrusion
  - `PatrolSector` and anchored guard / blockade / starbase defense are now
    documented as distinct layers
- the remaining salvage question is no longer gameplay legality; it is record
  decoding:
  - the recovered points do not obviously land in
    `PLAYER.DAT.player.stored_prod_pts_raw`
  - the owned planet record and matching `DATABASE.DAT` row do change
  - the changed bytes are not yet a clean plain-integer `+20` under current
    field assumptions
- exact classic `MESSAGES.DAT` mail/report format and routing semantics are
  still only partially recovered; current Rust behavior preserves classic mail
  but does not yet decode or reproduce it faithfully
- for the Rust client, do not infer "returning joined player" from
  `PLAYER.DAT` assigned-player fields alone:
  - live classic probing now shows caller/dropfile identity matching the
    persisted player handle is part of login recognition
  - keep a distinction between:
    - maint-valid fixtures
    - login-valid matching-player fixtures
    - matched pre-loaded first-login fixtures
  - Rust client startup should branch at least three ways:
    - first-time menu
    - matched pre-loaded player first-login onboarding
    - established joined-player login flow
  - future BBS-door dropfile support should stay Rust-native and forward-looking:
    - parse classic `CHAIN.TXT` plus modern telnet/BBS dropfile shapes through a
      thin `ec-client` session adapter layer
    - normalize those inputs into one internal Rust session/startup context
    - keep door-file parsing out of `ec-data` and core gameplay state
    - if the integration surface grows, split it into a thin launcher/adapter
      crate rather than pushing BBS-specific logic down into the engine
- SQLite-backed campaign persistence is now started:
  - each campaign uses a bundled/self-hosted `ecgame.db`
  - `ec-client` now loads/saves runtime state from `ecgame.db`
  - `maint-rust` now also runs against `ecgame.db` and stores its next
    snapshot there
  - classic `.DAT` import/export is now an explicit `ec-cli` bridge rather
    than the live runtime path for the client or Rust maintenance
  - for hybrid classic-client campaigns, `maint-rust` now refreshes SQLite
    from the live working directory before processing if classic `.DAT` files
    have changed since the last stored snapshot
  - Rust-created new games now seed `ecgame.db` automatically
  - the store keeps normalized record-set snapshots plus compatibility/export
    payloads for unresolved classic outputs
  - the total planet database now has a path for SQLite-backed `Last Intel`
    year metadata
  - intel tiers are now explicit:
    - `owned`
    - `full`
    - `partial`
    - `unknown`
  - current intel-year stamping is still first-pass and should be refined as
    more gameplay/report paths sync into SQLite

These are refinement tasks, not blockers for calling `rust-maint` a usable
full-game engine.

## Next Phase: SQLite-backed Rust ECGAME

The next major phase should be cloning `ECGAME` in Rust while keeping the
existing `.DAT` compatibility boundary intact.

Initial scope:

- replicate the player-facing command flow and reports, not just the maint
  backend
- use the existing Rust maintenance/report pipeline instead of recreating game
  rules in a second place
- use the SQLite campaign store as the first-class persisted campaign state
  while keeping `.DAT` import/export as the oracle compatibility boundary
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

Treat the login/startup side as one explicit pre-command-center pipeline:

- show the built-in EC ASCII splash first
- then show the in-client text intro pages
- after the intro, branch by player state before any command center opens
  - unjoined player:
    - first-time help/list/join flow
    - then First Time Menu
  - joined player:
    - reports/messages review
    - homeworld/new-colony naming prompts when applicable
    - then Main Menu
- keep this as one Rust client flow so onboarding, login-time review, naming,
  and menu entry are modeled together instead of as disconnected screens

## Immediate Next Steps

1. Probe the remaining client/login edge cases in live `ECGAME`:
   - verify the restored default `sysop new-game` path still triggers the full
     first-join naming/onboarding flow
   - distinguish clearly between:
     - unmatched caller -> first-time menu
     - matched pre-loaded player first login
     - established joined-player login
   - capture any remaining differences in report/message ordering or prompt
     wording between those three startup branches
2. Keep running periodic seeded multi-turn `rust-maint` sweeps to guard against
   regressions while the UI/client work begins.
3. Treat maint hardening as settled unless new evidence contradicts it:
   - do not weaken the shared-engine validation/sanitization path just to match
     older client-local behavior
   - if a future manual-only `A+` pass is desired, prove the remaining
     interpretation-heavy edges against original binaries before changing the
     current canonical Rust rules
4. Finish the fleet oracle pass before changing any manual-adjacent mission logic:
   - keep recording reproducible classic defects as known `v1.51` bugs instead
     of copying them into Rust by default
5. Tighten the remaining CLI/storage boundary:
   - identify which `ec-cli` mutators still operate directly on classic `.DAT`
   - decide which should become SQLite-native next and which should remain
     explicit compatibility tooling
   - keep the rule that only explicit CLI import/export paths bridge classic
     directories into the runtime
   - current intentional exception:
     `core-init-current-known-baseline` still mutates the projected `.DAT`
     directory directly because the canonical transition reports depend on its
     exact file-shape drift against the preserved post-maint baseline
6. Write a focused Rust `ECGAME` phase plan:
   - command center
   - reports and intel views
   - diplomacy screens
   - order-entry workflow
   - fleet mission target defaults:
     - combat missions should default to the closest known enemy world, not the
       player's homeworld
     - if no known enemy world exists, show a brief notice instead of opening a
       misleading target prompt
     - ETAC colonize targeting should later prefer the closest uncolonized
       planet, skipping the player's own worlds, skipping known colonized
       worlds, and avoiding planets already targeted by other friendly ETAC
       colonize missions, sorted by distance
   - defer real `X`/expert-mode behavior until the remaining command/menu
     surfaces are finished; implement it as a final menu-verbosity pass rather
     than a premature partial toggle
7. Keep tightening original production semantics for player-facing screens:
   - empire profile / rankings / planet info should use classic terms like
     `Present Production`, `Potential Production`, and `Total Available Points`
   - do not expose raw internal names like `factories` in the client UI
   - if stronger oracle evidence appears, refine the canonical Rust growth
     formula rather than reintroducing placeholder arithmetic
   - decode or rename the overloaded per-planet `raw[0x0E]` byte before using
     it for more player-facing economy output
8. Use the now-working DOSBox `ECGAME` harness to capture only the player-side
   screens and behaviors needed for the first Rust clone pass.
9. Continue the SQLite transition:
   - keep `ecgame.db` bundled/self-hosted with no external SQLite dependency
   - expand client/state sync so gameplay mutations refresh the latest snapshot
   - move more report/intel/history surfaces onto SQLite-backed queries
   - preserve `.DAT` export compatibility with oracle sweeps
10. Keep the total planet database aligned with the intel model:
   - all planets listed, fog-filtered
   - `?` for unknown fields
   - `Last Intel` year shown as `Y####` or `?`
   - Main/General remain intel views; Planet menus remain owned-asset views
