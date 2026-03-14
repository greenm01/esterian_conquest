# Preservation Approach

This repository is not trying to recover the original Pascal source code verbatim.

The goal is:

- preserve Esterian Conquest v1.5 as a working historical artifact
- reverse engineer its file formats and rules
- build Rust tooling that can generate 100% compliant gamestate files accepted
  by the original game and `ECMAINT`
- use that compliance target as the first concrete milestone toward a faithful
  modern reimplementation in Rust
- keep the original DOS binaries and data as the reference implementation

## Principles

1. Treat the original manuals as the semantic spec, and the DOS binaries as the
   compatibility oracle

- the shipped manuals define intended player-facing rules and mechanics
- `ECGAME.EXE` is the player-facing command UI
- `ECUTIL.EXE` is the sysop/configuration utility
- `ECMAINT.EXE` is the yearly maintenance and simulation engine
- when semantics and implementation quirks diverge:
  - prefer the manuals for gameplay meaning
  - prefer the binaries for file compatibility, accepted directory structure,
    and proven cross-file invariants
- do not chase byte-perfect parity if it would force Rust away from the
  original documented rules without adding compatibility value

2. Prefer confirmed behavior over guessed structure

- only name fields after they are supported by diffs, screenshots, docs, or repeated observation
- keep unknown bytes raw until they are mapped with confidence

3. Separate stable docs from lab notes

- `RE_NOTES.md` is the chronological investigation notebook
- `docs/` holds stable, reusable engineering docs

4. Keep the architecture layered

- `ec-data`: binary formats and typed accessors
- `ec-cli`: std-only scripting and verification interface
- keep the Rust implementation data-oriented:
  - explicit record/file data
  - focused free functions or small impl blocks
  - feature-oriented submodules instead of monolithic source files

5. Use fixtures to lock in behavior

- original shipped state
- initialized state
- post-maintenance state
- targeted scenario snapshots for specific features

6. Prefer engine outputs over UI playback

- `ECMAINT` writes the underlying state and generated report data
- `ECGAME` is still useful, but mainly as a viewer/validation layer for those outputs
- when possible, decode changes in `.DAT` files first and use live report viewing second
- historical text captures are reference evidence when live playback is unavailable or flaky

7. Use escalating RE depth, not maximum depth by default

- start with Rust-generated scenarios, preserved fixtures, and black-box
  `ECMAINT` acceptance testing
- promote repeated deterministic pass/fail patterns into shared Rust rules first
- escalate to deep static/dynamic RE only when all three are true:
  - a path is blocking broader compliant gamestate generation
  - black-box testing has plateaued
  - the expected rule is reusable, not one-off trivia
- when deep RE is required, stop once the rule is explicit enough to promote
  into Rust; do not keep digging only to satisfy curiosity
- treat the recent Guard Starbase / `unknown starbase` investigation as the
  template for a justified deep-dive blocker, not as the default workflow for
  every mechanic

8. Prefer controlled order -> `ECMAINT` -> diff loops for new mechanics

- initialize a controlled directory in Rust or from a preserved baseline
- submit one tightly scoped order family
- run `ECMAINT` as the oracle
- diff `.DAT`, `MESSAGES.DAT`, `RESULTS.DAT`, and `ERRORS.TXT`
- promote only repeated deterministic effects into `CoreGameData`
- use deep RE only after this loop stops yielding reusable rules

9. Treat setup and map generation as gameplay semantics, not scaffolding

- the manuals explicitly define galaxy size by player count and total solar
  system count
- the Rust builder is useful infrastructure, but it is not automatically the
  same thing as a faithful EC game initializer
- setup should therefore be refined as a manual-driven subsystem:
  - map dimensions
  - star count
  - homeworld/start rules
  - initial fleets and empire payloads
- exact reproduction of the original hidden map RNG is not required to be
  faithful; adherence to the documented setup rules is

10. Separate recovered mechanics from canonical routing policy

- movement execution rules should follow recovered deterministic behavior where
  known
- route selection and threat-aware navigation may be improved canonically in
  Rust when the manuals do not define a detailed routing algorithm
- smart pathfinding should be documented as a Rust policy layer, not implied to
  be a recovered original mechanic

11. Prefer declarative sysop config over endless setup flags

- `ECUTIL`-style setup/admin data is mostly declarative and should eventually
  live in KDL rather than only in one-off command flags
- the long-term source of truth for new-game/setup presets should therefore be
  machine-readable config:
  - player count
  - year
  - maintenance schedule
  - sysop options
  - optional map-generation seed
  - setup mode / starting-state presets
- CLI and future TUI surfaces should act as frontends over that config and the
  shared Rust model, not as the only place where setup can be expressed

## What Counts As Success

Short term:

- decode the important on-disk formats
- reproduce `ECUTIL` behavior faithfully
- understand `ECMAINT` as a deterministic state transformer
- define the cross-file invariants required for original-engine acceptance
- generate fully compliant gamestate files from Rust

Long term:

- reimplement the real turn engine in Rust
- build a usable player client and admin client
- support classic-compatible saves and reproducible results
- preserve the original player-facing ANSI presentation well enough to reuse
  or faithfully recreate important opening/menu/report screens in the Rust
  client
- eventually support both:
  - classic `.DAT` directory interchange with the DOS binaries
  - a richer modern storage layer where useful, likely through SQLite
    importing/exporting through the same canonical Rust state model

## Milestone Ladder

1. Known accepted scenarios

- Rust can emit preserved accepted pre-maint scenarios from decoded fields
- the original binaries and preserved fixtures are the acceptance oracle
- current examples:
  - `fleet-order`
  - `planet-build`
  - `guard-starbase`

2. Parameterized scenario generation

- replace scenario-specific constants with explicit field builders and
  validators
- move from "recreate this one accepted shape" toward "generate families of
  accepted shapes within known-safe constraints"

3. General compliant gamestate generation

- Rust can write a full arbitrary gamestate directory that `ECMAINT` accepts
  without integrity failures
- this requires the remaining cross-file linkage rules, especially the
  starbase/fleet summary-pairing semantics in `ECMAINT`

4. Full Rust maintenance replacement

- reimplement `ECMAINT` behavior in Rust with reproducible outputs
- preserve compatibility with original save directories and reports
- deterministic combat is now implemented as a canonical Rust replacement for
  the original RNG-driven combat paths
- combat acceptance is therefore structural and rule-based, not byte-exact to
  any one oracle run

5. Scenario DSL / KDL layer

- add a human-editable scenario/order format only after the internal Rust
  gamestate and order model stabilizes
- treat KDL as a serialization layer over the compliant generator, not as the
  next reverse-engineering target
- KDL is still a good long-term fit for stable machine-readable data:
  - combat/entity constants
  - setup and baseline presets
  - oracle scenario definitions
- Rust remains the authority for maintenance sequencing and classic save-file
  compatibility; config should feed stable data tables, not replace the engine
- future storage layers should follow the same rule: they may sit beside the
  classic `.DAT` flow, but not replace the compatibility boundary
- long-term goal:
  - describe scenarios
  - describe per-turn player orders
  - emit gamestate files
  - run original `ECMAINT`
  - iterate over a whole game from scripted inputs

6. ANSI / UI preservation layer

- capture and preserve the original `ECGAME` ANSI output/screens where
  practical
- treat those captures as reference assets for the Rust client
- prefer exact stream capture when possible and rendered-screen capture as a
  fallback
- this is not the immediate RE priority, but it is an explicit preservation
  goal and should be folded into the Rust clone once the local `ECGAME`
  harness is reliable enough

12. Own the mechanics; do not reproduce the original RNG

- `ECMAINT` uses an internal RNG for combat resolution (fleet battles,
  bombardment ship losses) and rogue/autopilot AI decisions
- the original RNG output is not reproducible without full emulation of its
  internal state; attempting to match it byte-for-byte is intractable and
  would produce a brittle clone, not a faithful reimplementation
- instead, implement **our own deterministic versions** of every mechanic:
  - use the original binary and preserved fixtures to understand the
    *structure* of changes (what fields change, in what range, under what
    conditions)
  - define our own canonical rules for the *magnitude* of random effects
    (e.g. bombardment ship losses, battle attrition rates, AI economy choices)
  - document those rules here and in `RE_NOTES.md` so they are auditable
    and tunable independently of the original binary
- the acceptance criterion for these mechanics is internal consistency and
  gameplay plausibility, not byte-exact fixture match
- byte-exact fixture match remains the acceptance criterion only for fully
  deterministic mechanics (movement, year advancement, build queues, economy
  totals, cross-file linking)
- the original post-maint fixtures are still used to understand field
  ranges and change patterns; they are not used as a bit-level target for
  stochastic mechanics
- once these canonical mechanics stabilize, prefer moving their stable
  constants into machine-readable KDL config rather than burying them inline
  forever in Rust code

13. Preserve compatible gamestate even when behavior is canonicalized

- the Rust engine is now far enough along that it should prefer
  **classic-compatible save directories** over brittle attempts to mimic every
  hidden stochastic or processing-order quirk of the original binaries
- for unresolved or stochastic mechanics, a documented canonical Rust rule is
  acceptable if:
  - the resulting `.DAT` files remain loadable and sane in original `ECGAME`
  - the resulting directories remain structurally acceptable to the original
    maintenance/tooling workflow
  - the rule is faithful to the player manuals and observed gameplay spirit
  - the rule is deterministic, auditable, and regression-testable
- this means:
  - file compatibility remains strict
  - deterministic mechanics should still match exactly where practical
  - non-deterministic or under-recovered mechanics may reasonably diverge when
    the divergence is explicit, compatible, and more reproducible than the
    original hidden behavior

14. Keep diplomacy and hostility separate

- `enemy` is a stored diplomatic relation set by players in `ECGAME`
- `hostile` is the broader maintenance/combat state that determines whether a
  contact may escalate into battle
- a contact can become hostile because:
  - one side has declared the other an enemy
  - one side attacks first
  - one side enters another empire's defended solar system
  - one side enters or leaves a blockaded world
- Rust should model the distinction in docs and code rather than collapsing
  both concepts into one permanent shortcut
- where classic `PLAYER.DAT` diplomacy bytes are known, they are authoritative
- a modern sidecar such as `diplomacy.kdl` is acceptable only as a fallback for
  player-count tiers or edge cases that the recovered classic layout does not
  yet cover
- the first recovered mapping is now live:
  - `PLAYER.DAT[player].raw[0x54 + (target_empire_raw - 1)]`
  - `0x00 = neutral`
  - `0x01 = enemy`

15. Treat surrender as campaign state, not an assumed `ECGAME` command

- the manuals describe surrender and acknowledgement of an emperor as the
  political victory condition
- the documented `ECGAME` General Command menu does not include a surrender or
  resign action
- a live `ECGAME` menu check now confirms that absence
- therefore Rust should not invent a surrender UI command unless stronger
  evidence appears
- the Rust model should instead separate:
  - mechanical defeat:
    - destruction of armies, fleets, and planets
    - fleet defection after loss of all planets
  - political victory:
    - recognition of one empire as emperor
    - effective surrender or submission of the remaining empires
- the contiguous layout from `0x54..=0x6C` now lets Rust treat that table as a
  25-slot diplomacy surface, matching the documented maximum player count

Near-term acceptance rule:

- a format/mechanic is not "done" until Rust can emit the relevant state and
  the original binaries accept it without integrity failures or unexpected
  normalization
- the original `ECMAINT` oracle is therefore a compatibility and structure
  oracle first, not a universal semantics oracle
- bit-perfect post-maint parity is worth pursuing only when it supports the
  manuals and the mechanic is deterministic enough for that target to be
  meaningful
- for stochastic mechanics, "done" means: correct field structure, plausible
  magnitudes, and a documented canonical rule — not byte-exact match to any
  single oracle run
- for manual-driven mechanics whose original binary behavior is ambiguous,
  opaque, or stochastic, strict adherence to the manuals is a better target
  than reproducing one hidden implementation artifact
- the combat spec in
  [docs/ec-combat-spec.md](/home/mag/dev/esterian_conquest/docs/ec-combat-spec.md)
  is no longer only aspirational; it now drives the live Rust maintenance path
  and has dedicated regression coverage

## RE Workflow

Default loop:

1. Generate or mutate a controlled scenario in Rust.
2. Run the original binary (`ECMAINT`, `ECGAME`, or `ECUTIL`) as the oracle.
3. Diff the resulting `.DAT` files and reports.
4. Promote only strong repeated patterns into `CoreGameData`.
5. Escalate to deep RE only if the rule still blocks generalization.

## Event And Report Direction

Maintenance-side player-visible consequences should be modeled as typed events
first, and rendered into classic report files second.

Current direction:

- `ec-data` owns the event surface emitted by maintenance mechanics
- `ec-cli` owns report-file regeneration (`DATABASE.DAT`, `RESULTS.DAT`, and
  later any justified `MESSAGES.DAT` writer)
- report formatting should not be embedded ad hoc inside mechanic code paths

This applies beyond combat. The same event/report pipeline should eventually
cover:

- fleet encounters and retreats
- bombardment, invasion, blitz, and starbase defense
- colonization success/failure
- scout reconnaissance and contact discovery
- mission completion / mission denial outcomes

Near-term policy:

- continue broadening the typed maintenance event surface
- keep pushing those events through a single report-generation pass
- use the generic mission-outcome backbone for first-pass scout arrival reports
  before implementing richer planet-intel reconnaissance reports
- let `ScoutSolarSystem` reuse the existing `PlanetIntelEvent` /
  `DATABASE.DAT` refresh path where the current maintenance model can already
  support it
- let `ViewWorld` use that same intel-refresh path rather than creating a
  separate report-only branch
- when combat forces a fleet off its standing orders, emit a typed mission
  `Aborted` outcome from the battle phase instead of hiding that consequence
  inside fleet-byte mutations alone
- let scout-style hostile contact detection be emitted from the battle/contact
  grouping phase, because that is where maint has the cleanest simultaneous
  view of who met whom before attrition rewrites the board
- keep that contact event family mission-aware so scout, join, rendezvous, and
  guard/blockade reports can share one detection path without copy-pasted
  reporting logic
- prefer recipient-scoped maintenance events over omniscient report summaries;
  bombardment, fleet battle, scouting/contact, merge, colonization, and mission
  outcome reporting should be modeled from the acting or affected empire's
  point of view rather than as a global debug narration
- let destructive combat consequences become first-class events too; fleets and
  starbases that are wiped out should emit explicit command-center loss reports
  rather than being inferred indirectly from missing units
- where richer specialized report events exist, prefer them over duplicate
  generic mission-resolution text; invade/blitz should not generate two
  parallel attacker-side reports for the same assault
- every fleet encounter should eventually emit an intel/contact event even if
  no battle occurs; combat is only one possible consequence of contact
- treat `RESULTS.DAT` as the active canonical maint report target
- leave `MESSAGES.DAT` empty until a non-empty maint-generated sample is
  recovered from fixtures, oracle runs, or historical session captures
- current limitation: the Rust port still writes one aggregate `RESULTS.DAT`
  stream for maintenance, even when the underlying events are recipient-scoped;
  exact classic per-player report routing remains a later report-layer task

Default `ECMAINT` black-box loop for new mechanics:

1. `python3 tools/ecmaint_oracle.py prepare <target_dir> [source_dir]`
2. submit one controlled set of orders or mutate one narrow field family
3. `python3 tools/ecmaint_oracle.py run <target_dir>`
4. inspect the `.oracle/` snapshots plus the printed diff clusters
5. promote only strong repeated rules into shared Rust logic

Known-scenario replay loop:

1. `python3 tools/ecmaint_oracle.py replay-known fleet-order /tmp/ecmaint-fleet-oracle`
2. inspect the `.oracle/` snapshots and the comparison against the preserved
   post-maint fixture
3. use the same pattern for `planet-build` and `guard-starbase` before opening
   a new mechanic

Preserved-fixture replay validation:

1. `python3 tools/ecmaint_oracle.py replay-preserved fleet-order /tmp/ecmaint-fleet-pre-direct`
2. confirm the preserved pre-maint fixture replays to the preserved post-maint
   fixture exactly
3. use `replay-known` to measure the remaining gap in the Rust-generated
   pre-maint state, not to question the oracle harness itself

Deep RE escalation criteria:

- use static/dynamic RE when a blocker survives repeated black-box tests
- prefer narrow, reproducible captures over broad exploratory tracing
- stop the deep dive once the missing rule can be stated precisely enough for
  Rust validation/generation

Anti-rabbit-hole rule:

- do not apply full deep-dive treatment to every mechanic
- if a path is not currently blocking broader compliant generation, keep it in
  the black-box queue until it becomes a real blocker

Current concrete Rust milestone:

- `ec-cli` is now split by command family instead of continuing to grow in
  one file:
  - `src/commands/fleet_order.rs`
  - `src/commands/planet_build.rs`
  - `src/commands/guard_starbase.rs`
  - `src/commands/ipbm.rs`
- start from a known-good preserved snapshot such as
  `fixtures/ecmaint-post/v1.5`
- rewrite only decoded fields in Rust
- verify the rewritten `.DAT` file matches a preserved accepted pre-maint
  scenario exactly
- current confirmed examples:
  - fleet-order rewrite reproducing `fixtures/ecmaint-fleet-pre/v1.5/FLEETS.DAT`
  - planet build-queue rewrite reproducing
    `fixtures/ecmaint-build-pre/v1.5/PLANETS.DAT`
  - named fleet/build scenario commands and validators over those same
    preserved accepted rewrites:
    - `ec-cli scenario <dir> fleet-order`
    - `ec-cli scenario <dir> planet-build`
    - `ec-cli validate <dir> fleet-order`
    - `ec-cli validate <dir> planet-build`
    - `ec-cli scenario-init [source_dir] <target_dir> fleet-order`
    - `ec-cli scenario-init [source_dir] <target_dir> planet-build`
  - accepted Guard Starbase scenario rewrite reproducing the core gamestate
    files from `fixtures/ecmaint-starbase-pre/v1.5`
  - the Guard Starbase base record is now emitted from named Rust field
    setters over `BaseRecord`, not from a preserved raw byte blob
  - Rust can also validate the currently-known accepted one-base Guard
    Starbase shape with `ec-cli validate <dir> guard-starbase`
  - that Guard Starbase validator is now explicitly linkage-shaped based on the kind-1 / kind-2 semantic keys:
    - player starbase-count word
    - fleet local-slot word matching base summary word
    - fleet ID word matching base chain word
    - guard target/base coords
    - guard starbase index/enable bytes
  - Rust also now has a first explicit parameterized starbase writer:
    - `ec-cli guard-starbase-onebase <dir> <target_x> <target_y>`
    - this explicit encoder derives the accepted base linkage keys directly from the `PLAYER.DAT` and `FLEETS.DAT` records instead of relying on a hard-coded accepted-shape blob
  - Rust also now has a direct linkage inspection command:
    - `ec-cli guard-starbase-report <dir>`
    - use this to print the currently relevant player/fleet/base key words
      before and after running `ECMAINT`
  - Rust also now has a one-command parameterized directory initializer:
    - `ec-cli guard-starbase-init [source_dir] <target_dir> <target_x> <target_y>`
    - this is the quickest current path from a known-good baseline to a new
      ECMAINT-ready one-base Guard Starbase experiment
  - Rust also now has a batch coordinate-variant starbase initializer:
    - `ec-cli guard-starbase-batch-init [source_dir] <target_root> <x:y> <x:y>...`
    - use this when you want multiple ECMAINT-ready one-base Guard Starbase
      experiments from one baseline in a single run
  - Rust also now has the first practical `IPBM.DAT` control surface:
    - `ec-cli ipbm-report <dir>`
    - `ec-cli ipbm-zero <dir> <count>`
    - `ec-cli ipbm-record-set <dir> <record_index> <primary> <owner> <gate> <follow_on>`
    - `ec-cli ipbm-validate <dir>`
    - `ec-cli ipbm-init [source_dir] <target_dir> <count>`
    - `ec-cli ipbm-batch-init [source_dir] <target_root> <count> <count>...`
    - this is enough to keep generated scenarios on the correct side of the
    known `PLAYER[0x48]` / `IPBM.DAT` count gate and to start emitting
    structured non-zero prefix fields without hand-editing bytes
    - `ec-data::IpbmRecord` now also exposes:
      - tuple A/B tag bytes
      - tuple A/B/C payload groups
      - trailing control bytes
    - `ec-cli ipbm-report` now prints those structural groups directly
  - Rust also now has a combined integrity-focused inspection command:
    - `ec-cli compliance-report <dir>`
    - `ec-cli compliance-batch-report <root>`
    - this prints the current Guard Starbase linkage verdict, current `IPBM`
      count/length verdict, and the most relevant key words in one pass
    - the batch form is meant for generated scenario roots and keeps the
      output compact enough to compare multiple experiment directories quickly
    - the batch form now covers:
      - `fleet-order`
      - `planet-build`
      - `guard-starbase`
      - `ipbm`
  - `ec-cli validate <dir> all` now gives a quick classification pass across
    the current known accepted scenarios
  - Rust now also has parameterized fleet/build inspection and init commands:
    - `ec-cli fleet-order-report [dir] [fleet_record]`
    - `ec-cli fleet-order-init <target_dir> <fleet_record> <speed> <order_code> <target_x> <target_y> [aux0] [aux1]`
    - `ec-cli fleet-order-batch-init <target_root> <fleet_record:speed:order:target_x:target_y[:aux0[:aux1]]>...`
    - `ec-cli planet-build-report [dir] [planet_record]`
    - `ec-cli planet-build-init <target_dir> <planet_record> <build_slot_raw> <build_kind_raw>`
    - `ec-cli planet-build-batch-init <target_root> <planet_record:build_slot_raw:build_kind_raw>...`
    - these now make the fleet/build paths consistent with the existing
      starbase/IPBM report/init workflow and remove more experiment setup from
      hand-edited fixture directories
    - the new batch forms now materialize multiple parameterized experiment
      roots plus manifests in one run
  - the known accepted scenarios are now centralized behind one Rust-side
    catalog:
    - `ec-cli scenario <dir> list`
    - `ec-cli scenario <dir> show <scenario>`
    - `ec-cli scenario-init-all [source_dir] <target_root>`
  - scenario validation now has two useful levels:
    - rule-shaped validators: `ec-cli validate <dir> ...`
    - preserved exact-match validators: `ec-cli validate-preserved <dir> ...`
  - preserved scenario drift can now be inspected directly with
    `ec-cli compare-preserved <dir> ...`
  - Rust can now materialize a runnable Guard Starbase directory from a
    compliant baseline with
    `ec-cli scenario-init [source_dir] <target_dir> guard-starbase`
  - the documented optional-source forms now work as intended:
    - `ec-cli init <target_dir>` defaults to `original/v1.5`
    - `ec-cli scenario-init <target_dir> <scenario>` defaults to
      `fixtures/ecmaint-post/v1.5`

This is intentionally narrower than a full arbitrary save generator, but it is
the first real proof that the Rust layer can emit accepted gamestate files from
decoded state rather than only copy fixture trees.

## Current Strategy

Near-term effort should prioritize `ECMAINT`.

Why:

- `ECUTIL` is mostly configuration/state setup
- `ECGAME` is mainly command entry and presentation
- `ECMAINT` appears to be the core simulation engine:
  - movement
  - battles
  - build completion
  - AI / rogue empire behavior
  - database and report generation

That makes `ECMAINT` the highest-value target for recovering the actual rules of the game.
It is also the main acceptance oracle for the first milestone: Rust-generated
gamestate files that are 100% compliant with the original engine.

## ECMAINT Investigation Model

The current `ECMAINT` workflow is:

1. create one controlled pre-maint scenario
2. run original `ECMAINT`
3. diff the resulting `.DAT` files
4. preserve pre/post fixtures
5. encode the confirmed transform in Rust tests
6. optionally read the generated reports through `ECGAME` as a validation step

This keeps the preservation work grounded in deterministic engine behavior rather
than in UI rendering.

Rust layout note:

- see `docs/rust-architecture.md` for the current submodule split and the
  data-oriented design guidance for future refactors

## Session Handoff

When pausing work, keep the immediate restart point in:

- `docs/next-session.md`

That file should be updated with:

- the latest high-confidence combat model
- the most recent commits worth resuming from
- the exact next controlled experiment
