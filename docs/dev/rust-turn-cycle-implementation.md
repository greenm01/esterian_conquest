# Rust Turn-Cycle Implementation Spec

This document is the implementation-facing companion to
[ec-turn-cycle-spec.md](ec-turn-cycle-spec.md).

Read it together with:

- [ec-combat-spec.md](ec-combat-spec.md)
  for mission/combat behavior
- [economics.md](economics.md)
  for the Rust economy/build policy that lives inside the post-loop
  world/player update region
- [ec-timing-spec.md](ec-timing-spec.md)
  for weekly scheduler and report `Stardate` behavior
- [rust-architecture.md](rust-architecture.md)
  for the repository-wide data-oriented/module-boundary rules this engine
  implementation should follow

Ownership boundary:

- this document owns yearly phase placement and turn-order boundaries
- [ec-combat-spec.md](ec-combat-spec.md)
  owns combat and hostile world-resolution mechanics
- [ec-timing-spec.md](ec-timing-spec.md)
  owns weekly timing, report-week assignment, and `Stardate` header formatting
- [economics.md](economics.md) owns
  the Rust economy/build policy inside the post-loop world/player update region

Use it when designing or refactoring the Rust maintenance engine.

This is not the raw RE notebook and not a byte-offset map. Its job is to
describe the turn-cycle as a practical engine/state-machine problem:

- which phases exist
- what each phase is responsible for
- what state each phase may read or write
- which boundaries are settled
- how the turn-order boundaries interact with the companion combat, economics,
  and timing specs

For raw oracle evidence and confidence notes, use the canonical turn-cycle
spec. For combat rules and timing/report formatting, defer to the dedicated
companion specs above.

## Scope

This document is for the Rust yearly maintenance engine, currently
`rust-maint`.

It deliberately focuses on:

- human-digestible process flow
- engine boundaries
- durable state vs transient workspaces
- report/event generation boundaries
- safe implementation seams for Rust

It deliberately avoids:

- low-level byte-field inventories
- speculative semantic naming for still-unknown fields

## One-Page Model

The current best implementation model is:

1. wait until maintenance is allowed to run
2. recover from interrupted movement if needed
3. load and validate the campaign directory as a coherent whole
4. run the yearly simulation core
5. canonicalize the generated summary/event pool
6. emit weekly reports and derived outputs
7. flush outputs, clean up, and end the tick

## Block Diagram

```text
Classic Directory / Rust Directory
    |
    v
+------------------------------+
| 1. Schedule / Token Gate     |
+------------------------------+
    |
    v
+------------------------------+
| 2. Recovery / Restore        |
|    if prior run halted       |
|    during movement           |
+------------------------------+
    |
    v
+------------------------------+
| 3. Load + Cross-File         |
|    Validation                |
+------------------------------+
    |
    v
+------------------------------+
| 4. Yearly Simulation Core    |
|                              |
| 4a. prepare workspaces       |
| 4b. annual movement update   |
| 4c. pre-loop fleet setup     |
| 4d. determine fleet visit    |
|     order (sort-by-random-   |
|      priority)               |
| 4e. 52-week fleet loop:      |
|     event scheduling,        |
|     combat, inline reports   |
| 4f. post-loop fleet scan     |
| 4g. post-loop world/player   |
|     update region:           |
|     build completion,        |
|     economy/autopilot,       |
|     player recompute         |
| 4h. producer/mutator +       |
|     assault/campaign passes  |
| 4i. database/header updates  |
+------------------------------+
    |
    v
+------------------------------+
| 5. Summary Canonicalization  |
|    / Matching / Sorting      |
+------------------------------+
    |
    v
+------------------------------+
| 6. Weekly Report Emission    |
|    + Derived Output Build    |
+------------------------------+
    |
    v
+------------------------------+
| 7. Final Flush / Cleanup     |
+------------------------------+
```

## Process Flow

```text
START
  |
  v
Check schedule and token files
  |
  +--> not allowed to run -> exit without simulation
  |
  v
Check crash marker for interrupted movement
  |
  +--> recovery needed -> restore .SAV over .DAT
  |
  v
Load files into a coherent game-state model
  |
  +--> validation failure -> abort with error outputs
  |
  v
Create / reset transient workspaces
  |
  v
Run annual movement update
  |
  v
Run pre-loop fleet setup if needed
  |
  v
Determine fleet visit order (sort-by-random-priority; Rust may use slot order)
  |
  v
52-week fleet-processing loop
  |
  +--> per week: process each fleet in visit order
  |    +--> read fleet, timing-window check
  |    +--> resolve fleet-vs-fleet combat + emit RESULTS.DAT inline
  |    +--> update weekly fleet event state
  |    +--> write fleet, remove destroyed/captured
  |
  v
Post-loop fleet summary scan (2 sequential reads)
  |
  v
Post-loop world/player update region:
  |
  +--> build completion
  |    +--> ships/starbases stage into stardock
  |    +--> armies/ground batteries apply directly to planet
  |
  +--> planet economy
  |
  +--> autopilot / rogue economy
  |
  +--> player planet-count / production recompute
  |
  v
Producer/mutator + assault/campaign passes
  |
  +--> hostile world-resolution family
  |    (bombard / invade / blitz / aftermath)
  |
  v
DATABASE.DAT / header updates
  |
  v
Canonicalize and sort summary/events
  |
  v
Walk internal weekly timeline and emit player-visible outputs
  |
  v
Rebuild derived files and final outputs
  |
  v
Cleanup tokens / work files and finish
  |
  v
END
```

## Target Rust Turn Order

This is the turn ordering Rust should implement to stay aligned with the
canonical `ECMAINT` recovery. This section is the spec target. It is more
authoritative than any temporary ordering visible in the current Rust code.

1. schedule/token gate
2. interrupted-run restore if needed
3. load and cross-file validate the campaign state
4. prepare transient workspaces
5. annual fleet movement update
6. pre-loop fleet setup / capture-reassignment work
7. determine fleet visit order
8. run the 52-week fleet-processing loop
   - timing-window checks
   - co-located fleet-vs-fleet combat
   - inline `RESULTS.DAT` emission
   - weekly fleet-state updates
9. post-loop fleet scan
10. post-loop world/player update region
    - build completion
    - ship/starbase builds stage into stardock
    - army/ground-battery builds apply directly to the planet
    - economy/autopilot updates
    - player production/count recomputation
11. late middle producer/mutator and hostile world-resolution region
    - bombard / invade / blitz / target-world aftermath family
12. database/header updates
13. canonicalize the event pool
14. emit weekly reports and derived outputs
15. final flush / cleanup

Combat location summary:

- fleet-vs-fleet combat is step `8`, inside the 52-week fleet loop
- build completion and economy are step `10`, after that fleet loop
- hostile world resolution is step `11`, later than both the fleet loop and the
  post-loop build/economy region

Recovered hostile-world rule:

- delayed hostile world missions do not mutate the target world on tick `1`
  after arrival
- ready hostile world missions select their target-world resolution family by
  mission kind (`BombardWorld` vs `InvadeWorld`) in the later hostile
  world-resolution region
- ready hostile world missions read post-build planet state

## Engine State Model

The Rust engine should model four distinct state layers.

| Layer | Meaning | Examples | Lifetime |
| --- | --- | --- | --- |
| Durable game state | The real campaign state that survives the turn | players, planets, fleets, bases, IPBMs, conquest/setup state | persisted |
| Transient staging/workspaces | Scratch collections used while validating or simulating | staged tables, temporary counters, intermediate working sets | one maintenance run |
| Durable summary/event pool | Intermediate event records that survive long enough to be canonicalized and turned into reports | summary/event entries later matched, sorted, and emitted | one maintenance run |
| Derived output projections | Files rebuilt or preserved at the compatibility boundary | rankings, database, preserved classic mail bytes, results | regenerated each run |

Practical rule:

- do not collapse these layers into one giant mutable pass
- especially do not mix:
  - validation scratch
  - durable simulation outcomes
  - late report formatting

## Recommended Rust Phase Map

| Phase | Responsibility | Reads | Writes | Confidence |
| --- | --- | --- | --- | --- |
| 1. Gate | Decide whether maintenance may run now | schedule config, token files | token/work status only | High |
| 2. Recovery | Recover from interrupted movement | crash marker, `.SAV` backups | restored durable files | High |
| 3. Load/validate | Build coherent in-memory state and reject impossible directories | all core `.DAT` files | validated in-memory model, error outputs on failure | High |
| 4. Simulation core | Apply the yearly game rules | validated state, staged work data, existing orders | durable game state, durable summary/event pool | Medium |
| 5. Canonicalize events | Match, coalesce, sort, and normalize event records | durable summary/event pool | canonicalized event pool | High |
| 6. Emit outputs | Convert canonical events into classic results and rebuild/preserve derived files | canonical event pool, durable state | `RESULTS.DAT`, preserved `MESSAGES.DAT`, rankings, database | High |
| 7. Flush/cleanup | Finish the tick cleanly | work markers, generated outputs | final files, token cleanup | High |

## Recommended Rust Subsystems

The Rust engine should stay split by responsibility, not by one giant
"maintenance" function.

| Subsystem | Responsibility |
| --- | --- |
| Gate/recovery | schedule check, token coordination, interrupted-run recovery |
| Loader/validator | file loading, cross-file linkage checks, structural normalization |
| Simulation driver | orchestration of yearly simulation subphases |
| Movement/contact/combat | fleet motion, encounters, combat outcomes, retreat-vs-hold routing for invalidated missions, retargets |
| Producer passes | state-mutator/event-producer families inside step `4` |
| Event pool | typed durable summary/event records |
| Canonicalizer | matching/coalescing/sorting of event records |
| Report emitter | weekly timeline walk and player-visible message generation |
| Derived output builder | rankings, database, other rebuilt non-message artifacts |

## Step 4: What Rust Should Assume Today

Step `4` is now substantially recovered. The right Rust posture is:

- implement it as a structured sequence of subphases
- movement is annual, the 52-week loop is event scheduling
- fleet-vs-fleet combat happens inline during the weekly loop
- hostile world resolution happens later, after the post-loop build/economy region
- economy/producer passes run after the fleet loop

### Step 4 Block Diagram

```text
Validated Durable State
    |
    v
+------------------------------+
| 4a. Prepare workspaces       |
+------------------------------+
    |
    v
+------------------------------+
| 4b. Annual movement update   |
|     (one-time position       |
|      advance for all fleets) |
+------------------------------+
    |
    v
+------------------------------+
| 4c. Pre-loop fleet setup     |
|     (captures/reassignments; |
|      skipped if none needed) |
+------------------------------+
    |
    v
+------------------------------+
| 4d. 52-week event scheduling |
|     loop (NOT physics sim)   |
+------------------------------+
    |
    +--[for week 1..52]------+
    |                         |
    |  for each fleet in      |
    |  visit order:           |
    |                         |
    |  +--> read fleet record |
    |  |                      |
    |  +--> timing-window     |
    |  |    check: events to  |
    |  |    emit this week?   |
    |  |                      |
    |  +--> if co-located     |
    |  |    hostile: resolve  |
    |  |    fleet-vs-fleet    |
    |  |    combat + emit     |
    |  |    RESULTS.DAT       |
    |  |    inline            |
    |  |                      |
    |  +--> update weekly     |
    |  |    event state       |
    |  |                      |
    |  +--> write fleet       |
    |       record            |
    |                         |
    +-------------------------+
    |
    v
+------------------------------+
| 4e. Post-loop fleet scan     |
+------------------------------+
    |
    v
+------------------------------+
| 4f. Post-loop world/player   |
|     update region:           |
|     build completion         |
|     ships/starbases ->       |
|     stardock                 |
|     armies/batteries ->      |
|     planet                   |
|     then economy/autopilot   |
|     and player recompute     |
+------------------------------+
    |
    v
+------------------------------+
| 4g. Producer/mutator +       |
|     assault/campaign passes  |
|     (planet state, durable   |
|      events, campaign state) |
+------------------------------+
    |
    v
+------------------------------+
| 4h. DATABASE.DAT / header    |
|     updates                  |
+------------------------------+
    |
    v
Updated Durable State + Durable Event Pool
```

### What Is Settled

| Point | Practical meaning for Rust |
| --- | --- |
| **Movement is annual, not per-week** | fleet positions are updated once per year (storing fractional travel state in tuple_c for multi-year journeys). Keep movement as a distinct pre-loop subphase |
| **Mission resolution requires start-of-year position** | bombard, colonize, invade resolve only when the fleet is at its target at the start of the year. Co-located fleets resolve within the same tick |
| **The 52-week loop is event scheduling, not physics** | the loop schedules encounter detection, combat resolution, and report emission from post-movement positions. Stardates come from timing codes, not physical arrival time |
| **Timing-window constants are recovered** | the scheduler constants are recovered; kind-1 producer assignment is recovered for codes `3..6`; code `7` is the decoder-local `IPBM` timing class; code `8` is an unfed consumer-side case in the preserved image. Only starbase fleets get a delayed producer-side timing offset |
| **Fleet visit order is sort-by-random-priority** | Classic assigns `Random(N)+1` to each fleet as a sort key (extraction: `(seed>>16) % N`), then processes in ascending key order. The Range `N` is dynamic per player. Exact replication requires the full PRNG call chain from validation, which is infeasible. **Rust uses deterministic slot order**, which produces byte-identical results against the oracle for all tested scenarios |
| **The weekly fleet loop is a real 52-pass processing loop** | treat the yearly core as 52 stable weekly passes over the active fleet set, with the set shrinking only when fleets are destroyed or captured |
| **Combat reports emitted inline during weekly loop** | RESULTS.DAT writes happen inside the fleet pass. Do not defer all report generation to a post-simulation phase |
| **Fleet-vs-fleet combat triggered by first co-located hostile fleet** | the engine reads the opposing fleet, resolves fleet combat, emits reports inline, then writes back. Opposing fleet's writeback happens later in the same pass |
| **Some hostile world-resolution paths can destroy stardock contents** | preserved evidence shows at least bombardment-side hostile resolution can remove planet-owned stardock contents on the target world. Rust must model those losses as real planet-state mutation and mirror the matching player-facing turn reports, but should not overclaim the exact stardock-damage mechanics yet |
| **Ready hostile world-resolution family is mission-specific** | replayable oracle probes from the same target world show delayed bombard/invade fleets leave the world unchanged on tick `1`, while ready `BombardWorld` and ready `InvadeWorld` produce different target-world mutation families. Model this as a ready mission-family dispatch in the late hostile world-resolution region |
| **Ready bombard/invade hostile resolution already sees completed builds** | paired oracle probes show a target-world build queue that becomes stardock inventory in delayed control is already consumed before ready `BombardWorld` or ready `InvadeWorld` world resolution. Rust may therefore treat ready hostile world-resolution as reading post-build world state |
| **Fleet destruction/capture dynamic** | destroyed fleets dropped from subsequent passes; captured fleets change ownership mid-simulation |
| **Pre-loop fleet setup phase** | fleet-battle has 5 pre-loop passes for captures/reassignments; non-combat scenarios skip this entirely |
| **Colonization is atomic on arrival** | ownership, armies (=1), name, status, production all set in one pass; economy starts the following tick |
| **Economy/autopilot is explicitly after the weekly fleet-combat loop** | this phase runs after the 52-week fleet loop and reads post-fleet-combat state before the later hostile world-resolution region |
| **Recovered classic economy/autopilot gate is `player[0] == 0xFF`** | the currently recovered classic pass applies to rogue-mode empires; civil-disorder `0x00` empires are frozen |
| **Current Rust also does broader post-loop planet/economy recompute work here** | the Rust engine currently places normal planet-economy updates and per-player production/count recomputation in the same post-loop world/player region, even though the oracle-backed classic gate currently only settles the `0xFF` autopilot/rogue pass |
| **File write ordering is stable** | FLEETS → RESULTS → DATABASE → PLAYER → PLANETS → CONQUEST → RANKINGS |
| **Administrative summaries share the weekly event stream** | contact/combat summaries and at least some follow-on administrative consequences belong to one timed event stream, not a detached year-end appendix |
| **`00e8/024d` are yearly producer passes with internal order** | they are part of the yearly producer family, not late report helpers; `f71d -> dddb` runs before `e31b`, and `024d` mixes real planet mutation with durable event production, sometimes silently |
| **The `861d` tail is late report/output work** | treat `1da6 / 0c06 / 2db3 / 56be` as late output/report processing, not as the place to infer gameplay-core phase order |

### Residual Uncertainty

Turn-order placement is now recovered strongly enough for implementation.

Mission/combat behavior is specified separately in
[ec-combat-spec.md](ec-combat-spec.md).
This document is only the turn-order/phase-boundary companion.

## Current Practical Step-4 Shape

The current best implementation shape for step `4` is:

```text
4a. Prepare transient simulation workspaces
4b. Annual movement update (one-time position advance for all fleets;
    store fractional travel state in tuple_c for multi-year journeys)
4c. Pre-loop fleet setup (captures/reassignments; skipped if none needed)
4d. Determine fleet visit order (sort-by-random-priority; Rust may use
    deterministic slot order)
4e. For each week 1..52 (EVENT SCHEDULING, not physics):
      For each fleet in visit order:
        - read fleet record
        - timing-window check: does this fleet have events to emit this week?
        - if co-located hostile: resolve fleet-vs-fleet combat + emit
          RESULTS.DAT inline
        - update weekly event state in fleet record
        - write fleet record
      Remove destroyed/captured fleets from active set
4f. Post-loop fleet scan (2 sequential reads of all fleet records)
4g. Post-loop world/player update region:
    - build completion
      - ships/starbases stage into stardock awaiting commission
      - armies/ground batteries apply directly to the planet and do not
        pass through stardock
    - normal planet economy updates
    - autopilot / rogue economy updates
    - per-player planet-count / production recomputation
4h. Producer/mutator + hostile world-resolution + assault/campaign region
    - bombard / invade / blitz / target-world aftermath family
4i. DATABASE.DAT / header updates
```

Key structural evidence:

- **movement is annual**: fleet positions update once per year, not
  per-week. Tuple_c (+0x19..+0x1E) stores Real48 fractional travel state
  for multi-year journeys (set during movement, cleared on arrival)
- **the 52-week loop is event scheduling**: stardates come from timing
  codes (+2/+7/+21/+30 week offsets), not from physical arrival time.
  A speed-3 fleet traveling 1 sector shows contact at week 50 (timing-code
  scheduled), not week 19 (physical arrival)
- the yearly core is a **real 52-pass fleet-processing loop** over the
  active set; non-combat scenarios show stable per-pass visit order, while
  combat/capture scenarios shrink the active set dynamically
- non-combat fleet processing is exactly 4 I/O events per fleet per pass:
  seek, read, seek, write
- fleet-vs-fleet combat processing adds extra reads of opposing fleet(s) and inline
  RESULTS.DAT writes
- at least some hostile world-resolution paths can mutate the target world's
  stardock contents, and the corresponding stardock-loss reports belong in the
  same player-visible turn-report stream
- immediate co-located bombard-side hostile resolution already sees target-world
  build completion output in the same yearly tick
- current Rust build completion semantics are explicit:
  - ships and starbases stage into stardock awaiting later commission
  - armies and ground batteries apply directly to the planet and never enter
    stardock
- PLANETS.DAT is **never accessed** during the 52-pass fleet loop; planet
  economy/production changes happen after the fleet loop
- in both the recovered spec and current Rust structure, economy-facing world
  updates are therefore **after the weekly fleet-combat loop**, not interleaved
  inside that loop
- the remaining hostile world-resolution family is later than the weekly
  fleet-combat loop and should not be collapsed into the same “combat” box in
  diagrams
- after the fleet loop, 2 sequential reads of all fleet records occur
  (post-loop summary scan)
- the flush order: PLAYER → PLANETS → CONQUEST → RANKINGS

## The `024d` Implication For Rust

The most important recent finding is that at least one producer pass is not
just "report prep." It mutates real world state and also contributes durable
events.

Implementation consequence:

- Rust should have a first-class concept of a producer pass that may do both:
  - mutate durable game state
  - append durable event records
- preserve producer ordering inside that family:
  - `f71d -> dddb` kind-1 work happens before later `e31b` kind-2 emission
  - `024d` is not an unordered late helper; it is part of the simulation core

That means the engine should not be shaped like this:

```text
simulate all durable state
then
generate all events from the finished state
then
format reports
```

It should instead allow this:

```text
run subphase
  -> mutate state
  -> emit durable event records

run later subphase
  -> mutate more state
  -> emit more durable event records

canonicalize the resulting event pool
emit reports from the canonicalized pool
```

## Shared Write Ownership Inside Step 4

The current practical evidence is strong enough to guide one more engine rule:

- do not model step `4` as a set of fully disjoint passes where each subphase
  owns a separate slice of world state
- some neighboring subphases appear to touch overlapping target-world state
- in at least some probes, a producer-side branch can overwrite target-world
  changes that also appear in natural hostile-resolution cases

Implementation consequence:

- keep step-`4` world updates ordered and explicit
- avoid hidden mutation from helper calls that makes overwrite order hard to
  audit
- prefer one of these shapes:
  - subphase functions that write directly in known order
  - or subphase-local change sets that are applied in known order

Avoid this shape:

```text
collect a bag of unordered world mutations from many helpers
merge them later with no explicit precedence
```

Prefer this shape:

```text
run hostile-resolution subphase
  -> apply its target-world updates

run producer/mutator subphase
  -> apply its target-world updates
  -> allow documented overwrite where needed
```

That keeps the engine faithful to current evidence without hiding precedence
inside unordered helper effects.

## Weekly Event Stream Implication

Current spec evidence is now strong enough to guide one more implementation
rule:

- contact/combat reporting and at least some administrative follow-on
  consequences share the same weekly event stream
- do not treat Fleet Command Center loss summaries or retarget consequences as
  a detached end-of-year appendix by default

Implementation consequence:

- the event pool and weekly scheduler should be capable of carrying both
  hostile-contact/combat outcomes and later administrative consequences in the
  same timed stream
- when a hostile world-resolution path destroys stardock contents, emit the
  matching player-facing losses through that same event/report path rather
  than as a detached late summary
- standing mission/status families can also recur across years:
  - fleets already in extended orbit commonly emit `orbit-world` again at week
    `1` of later years
  - standing bombardment can resume with an immediate week-`1` bombing report
    after hostile contact when the fleet was already in bombard posture at
    round start

## Concrete Scheduler-Family Summary

This is the Rust-facing summary of the concrete report-family placement rules
already strong enough to encode. For corpus evidence and edge cases, use
[ec-timing-spec.md](ec-timing-spec.md).

| Report family / transition | Observed weekly placement | Rust implication |
| --- | --- | --- |
| `sensor-contact -> identified` | same week consistently in the shipped corpus | emit these as an ordered same-week pair; do not force a week advance between contact and identification |
| `identified -> intercepted` | same week where directly chained | direct interception can continue inside the same weekly batch |
| `entered-system -> attacked` | same week or next week | treat attack timing as a separate hostile-contact event in the shared weekly stream; do not derive it mechanically from system-entry week |
| `identified -> orbit-world` | same source/year gaps `0`, `1`, or `4` in the shipped corpus; the zero-gap cases are all week `1` | treat `extended orbit` as a standing mission/status family; fleets already orbiting at round start may emit a week-`1` orbit report in the same yearly stream |
| `orbit-world -> sensor-contact` | wide-gap periodic same-source family | while extended orbit persists, later `sensor-contact` is an independent weekly-stream event driven by hostile presence/traffic, not by one internal orbit timer |
| `attacked -> bombing-run` | same source/year gaps `0`, `5`, `6`, or `7` in the shipped corpus; the zero-gap case is week `1` | standing bombardment can continue after hostile contact without one fixed delay table; support same-year continuation and the round-start immediate variant |
| `intercepted -> bombing-run` | one direct same-source case at gap `6` | generalize the bombardment continuation rule to hostile encounter during standing bombardment, not only to the literal `attacked` wording |
| `identified -> Fleet Command Center fleet-lost` | same-week cross-source interleaving is common but not universal | treat loss summaries as separate weekly-stream events, not as same-source mission progression |
| `attacked -> Fleet Command Center fleet-lost` | next-week cross-source interleaving in the observed corpus | do not force immediate same-week loss-summary emission after every attack report |
| `fleet-lost -> join-retarget` | same-week cross-source interleaving is observed | administrative retarget consequences can share the same weekly stream as the loss summary |
| `fleet-lost -> planet-bombarded` | same-week cross-source interleaving is observed, but delayed variants also exist | bombard aftermath belongs to the same scheduler stream, but not as one fixed same-source delay rule |

## Target-World Resolution Families

Current oracle evidence now supports a tighter implementation rule than before.

Replayable bombard/invade probes from the same generated target world show:

- delayed `BombardWorld` / `InvadeWorld` fleets leave the target world
  unchanged on tick `1`
- ready `BombardWorld` applies the bombardment damage family
- ready `InvadeWorld` applies a different ground-assault family
- both ready families read post-build planet state

Implementation consequence:

- do not route these world effects through the weekly fleet-combat box
- instead, dispatch them in the later hostile world-resolution region by
  ready mission kind
- keep the damage mechanics/report phrasing inside mission-specific helpers,
  but keep the phase boundary itself explicit and shared

## Recommended Driver Skeleton

This is the current recommended engine shape for `rust-maint`, updated to
reflect the weekly fleet-processing loop structure recovered from file-I/O
trace analysis.

```text
run_turn(directory):
  gate_result = run_gate_and_recovery(directory)
  if gate_result.skip:
      return

  state = load_and_validate(directory)

  work = create_turn_workspaces(state)
  events = create_event_pool()

  // Phase 4b: annual movement (one-time position update)
  move_all_fleets(state)  // updates positions, stores tuple_c travel state

  // Phase 4c: pre-loop fleet setup (captures/reassignments)
  run_fleet_setup(state)  // skipped if no fleets need reassignment

  // Phase 4d: determine visit order
  fleet_order = compute_fleet_visit_order(state)  // sort-by-random-priority;
                                                  // Rust may use slot order

  // Phase 4e: 52-week event scheduling loop (NOT physics sim)
  for week in 1..=52:
      for fleet in fleet_order.active_fleets():
          process_fleet_week(state, fleet, week, events)
          // inner body: read fleet, timing-window check,
          // fleet-vs-fleet combat if hostile, update weekly state,
          // emit fleet-loop reports inline, then write fleet
      fleet_order.remove_destroyed_and_captured(state)

  // Phase 4f: post-loop fleet summary scan
  scan_all_fleets_for_summary(state, events)

  // Phase 4g: post-loop world/player update region
  run_build_completion(state)
  // ships/starbases -> stardock awaiting commission
  // armies/ground batteries -> planet counts directly
  run_planet_economy(state)
  run_economy_autopilot(state)
  recompute_player_planet_stats(state)

  // Phase 4h: producer/mutator + hostile world-resolution + assault/campaign region
  run_planet_producer_passes(state, work, events)
  run_planetary_assaults_and_campaign_updates(state, events)
  // ready hostile world resolution mutates target planets and emits any
  // matching planet-side loss reports, including stardock-content losses

  // Phase 4i: database / header updates
  update_database_and_headers(state)

  // Phases 5-7
  canonicalize_events(events)
  emit_reports(state, events)
  rebuild_derived_outputs(state, events)
  flush_and_cleanup(directory, state, events)
```

Use that as the current implementation skeleton for the recovered turn order.
Keep caution around current-code drift, not around the recovered phase
placement itself. For combat mechanics, defer to
[ec-combat-spec.md](ec-combat-spec.md).

## Current Rust Driver Snapshot

This is a non-authoritative snapshot of the current refactored Rust maint
driver. Keep it separate from the target turn order above. If the code and the
spec disagree, the spec wins and the code should move. The same is true if this
snapshot drifts from the companion combat, timing, or economics specs.

1. advance the game year
2. merge co-located friendly fleets before movement
3. sanitize invalid player inputs
4. refresh retarget / seek-home / join-host / guard-starbase targets
5. process fleet movement
6. process mission-fleet merging
7. resolve fleet battles
   - if hostile action strips the ship class that makes the mission possible,
     abort the mission immediately
   - fleets that still hold the local field after that combat abort hold in
     place; fleets that do not hold the field retreat / seek home
8. apply colonization
   - only from current post-combat fleet state; do not execute stale
     pre-combat colonization arrivals after ETAC loss or forced retreat
9. process build completion
   - ship/starbase builds stage into stardock
   - army/ground-battery builds apply directly to the planet
10. run normal planet economy
11. run autopilot / rogue planet updates
12. recompute per-player planet count / production totals
13. apply campaign-state transitions and related player/fleet consequences
14. update player starbase flags
15. resolve ready planetary assaults
   - revalidate assault fleets against current post-combat state before
     bombard / invade / blitz execution; do not execute stale ready snapshots
16. apply join-host updates
17. normalize conquest header fields
18. assemble maintenance events and apply stored diplomatic escalations

That is the ordering the Rust diagrams should describe when they are trying to
show the current driver, not just the recovered classic outer boundaries.

## Allowed Writes By Phase

This table is the practical guardrail for implementation.

| Phase | May write durable game state | May write durable event pool | May write player-visible reports | May rebuild derived outputs |
| --- | --- | --- | --- | --- |
| Gate | No | No | No | No |
| Recovery | Yes, by restore only | No | No | No |
| Load/validate | No durable gameplay mutation | Validation scratch only | Error outputs only on failure | No |
| Simulation core | Yes | Yes | Usually no direct final report emission | Sometimes indirectly, but prefer deferring |
| Canonicalization | No new gameplay mutation | Yes, normalize existing events | No | No |
| Report emission | No new gameplay mutation | No new core events | Yes | Yes |
| Flush/cleanup | No new gameplay mutation | No | Final file writes only | Final file writes only |

## Report And Timing Model

Rust should treat report generation as a separate consumer of already-produced
events, not as the place where gameplay outcomes are decided.

### Report Flow Diagram

```text
Simulation subphases
    |
    v
Durable event pool
    |
    v
Canonicalize / match / sort
    |
    v
Weekly timeline walk (1..52)
    |
    +--> RESULTS.DAT
    +--> MESSAGES.DAT
    +--> rankings / database / other derived outputs
```

Practical rule:

- a state mutation may be real even if it produces no immediate visible report
- conversely, late report helpers are not evidence for gameplay-core ordering

## State-Machine View

```text
Idle
  |
  v
GateChecked
  |
  +--> Skipped
  |
  v
RecoveredOrClean
  |
  v
Validated
  |
  v
Simulating
  |
  v
EventsCanonicalized
  |
  v
ReportsEmitted
  |
  v
Flushed
```

## Implementation Rules

1. Keep validation outside the simulation core.
2. Keep the event pool as a first-class runtime structure.
3. Allow simulation subphases to mutate state and emit events in the same pass.
4. Keep canonicalization separate from event production.
5. Keep report emission separate from gameplay mutation.
6. Keep step `4` subphases explicit and ordered.
7. Treat mission-family timing as data/logic attached to the scheduler, not as
   one universal post-combat delay.
8. Prefer typed event records and explicit phase functions over giant
   cross-cutting mutation code.

## What This Document Does Not Claim

This document does not claim:

- the full mission/combat rulebook; use
  [ec-combat-spec.md](ec-combat-spec.md)
  for that
- the semantic meaning of every still-raw planet/player field
- the original combat RNG or full Pascal-era implementation structure

It does claim:

- the major outer turn-cycle boundaries are strong enough to guide Rust
- **movement is annual** (one-time position update), not per-week
- **the 52-week fleet loop is event scheduling**, not physics simulation
- the yearly simulation core is a real 52-pass fleet-processing loop
- stardates come from timing codes (+2/+7/+21/+30), not physical arrival
- kind-1 timing-code production for codes 3-6 is recovered; code 7 is
  decoder-local `IPBM`, and code 8 is an unfed consumer-side case
- fleet visit order is sort-by-random-priority, with Rust free to use slot
  order pragmatically
- mission resolution requires start-of-year position
- the recovered economy/autopilot phase is after the weekly fleet-combat loop
  and before the later hostile world-resolution region
- the currently recovered classic gate for that pass is `player[0] == 0xFF`
- colonization is atomic on arrival
- combat reports are emitted inline during the weekly loop
- at least some hostile world-resolution paths can destroy stardock
  contents and must produce matching player-facing turn reports
- ready hostile world-resolution is selected by mission family in the later
  hostile world-resolution region
- build completion precedes ready hostile world-resolution in the recovered
  bombard/invade probes
- producer/mutator passes are part of gameplay state mutation, not just
  report formatting
- the late `861d` tail is output/report oriented, not gameplay-core ordering
- event production, canonicalization, and report emission are distinct
  responsibilities and should stay distinct in Rust

## Relationship To Other Docs

- use [ec-turn-cycle-spec.md](ec-turn-cycle-spec.md)
  for oracle-backed phase evidence
- use [ec-combat-spec.md](ec-combat-spec.md)
  for mission/combat behavior and hostile world-resolution mechanics
- use [economics.md](economics.md)
  for the canonical Rust economy/build policy applied in the post-loop
  world/player update region
- use [ec-timing-spec.md](ec-timing-spec.md)
  for weekly report/timing evidence and `Stardate` report-header shape
- use [approach.md](approach.md)
  for project-level preservation and RE policy
- use [rust-architecture.md](rust-architecture.md)
  for codebase structure and DOD rules
