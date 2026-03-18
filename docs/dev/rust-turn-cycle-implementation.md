# Rust Turn-Cycle Implementation Spec

This document is the implementation-facing companion to
[ec-turn-cycle-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-turn-cycle-spec.md).

Use it when designing or refactoring the Rust maintenance engine.

This is not the raw RE notebook and not a byte-offset map. Its job is to
describe the turn-cycle as a practical engine/state-machine problem:

- which phases exist
- what each phase is responsible for
- what state each phase may read or write
- which boundaries are settled
- which parts of step `4` remain open for refinement

For raw oracle evidence and confidence notes, use the canonical spec.

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
| 4b. compute fleet visit      |
|     order (PRNG shuffle)     |
| 4c. 52-week fleet loop:      |
|     movement, contact,       |
|     combat, inline reports   |
| 4d. post-loop fleet scan     |
| 4e. economy/autopilot pass   |
|     (rogue empires only)     |
| 4f. producer/mutator passes  |
| 4g. database updates         |
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
Compute fleet visit order (PRNG shuffle seeded from game state)
  |
  v
52-week fleet-processing loop
  |
  +--> per week: process each fleet in visit order
  |    +--> read fleet, check co-located hostiles
  |    +--> resolve combat + emit RESULTS.DAT inline
  |    +--> update fleet state (movement, orders)
  |    +--> write fleet, remove destroyed/captured
  |
  v
Post-loop fleet summary scan (2 sequential reads)
  |
  v
Economy/autopilot pass (rogue empires only, player[0]=0xFF)
  |
  v
Producer/mutator passes (planet state, durable events)
  |
  v
DATABASE.DAT planet-specific updates
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

## Engine State Model

The Rust engine should model four distinct state layers.

| Layer | Meaning | Examples | Lifetime |
| --- | --- | --- | --- |
| Durable game state | The real campaign state that survives the turn | players, planets, fleets, bases, IPBMs, conquest/setup state | persisted |
| Transient staging/workspaces | Scratch collections used while validating or simulating | staged tables, temporary counters, intermediate working sets | one maintenance run |
| Durable summary/event pool | Intermediate event records that survive long enough to be canonicalized and turned into reports | summary/event entries later matched, sorted, and emitted | one maintenance run |
| Derived output projections | Files rebuilt from durable state and event results | rankings, database, routed messages, results | regenerated each run |

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
| 6. Emit outputs | Convert canonical events into reports/messages and rebuild derived files | canonical event pool, durable state | `RESULTS.DAT`, `MESSAGES.DAT`, rankings, database | High |
| 7. Flush/cleanup | Finish the tick cleanly | work markers, generated outputs | final files, token cleanup | High |

## Recommended Rust Subsystems

The Rust engine should stay split by responsibility, not by one giant
"maintenance" function.

| Subsystem | Responsibility |
| --- | --- |
| Gate/recovery | schedule check, token coordination, interrupted-run recovery |
| Loader/validator | file loading, cross-file linkage checks, structural normalization |
| Simulation driver | orchestration of yearly simulation subphases |
| Movement/contact/combat | fleet motion, encounters, combat outcomes, retreats, retargets |
| Producer passes | state-mutator/event-producer families inside step `4` |
| Event pool | typed durable summary/event records |
| Canonicalizer | matching/coalescing/sorting of event records |
| Report emitter | weekly timeline walk and player-visible message generation |
| Derived output builder | rankings, database, other rebuilt non-message artifacts |

## Step 4: What Rust Should Assume Today

Step `4` is now substantially recovered. The right Rust posture is:

- implement it as a structured sequence of subphases
- movement is annual, the 52-week loop is event scheduling
- combat resolution happens inline during the weekly loop
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
    |  PRNG visit order:      |
    |                         |
    |  +--> read fleet record |
    |  |                      |
    |  +--> timing-window     |
    |  |    check: events to  |
    |  |    emit this week?   |
    |  |                      |
    |  +--> if co-located     |
    |  |    hostile: resolve   |
    |  |    combat + emit     |
    |  |    RESULTS.DAT       |
    |  |    inline             |
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
| 4f. Economy/autopilot pass   |
|     (rogue empires only,     |
|      reads post-combat state)|
+------------------------------+
    |
    v
+------------------------------+
| 4g. Producer/mutator passes  |
|     (planet state + durable  |
|      event creation)         |
+------------------------------+
    |
    v
+------------------------------+
| 4h. DATABASE.DAT updates     |
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
| **Timing system fully recovered** | only codes 3-6 are ever produced (starbase→3 +21wk, BS→4 immediate, CA/TT/army→5 immediate, scout/DD→6 immediate). Codes 1,2,7,8 in the `a26e` switch are dead code — never assigned by any producer (confirmed by full binary search). Only starbase fleets get a delayed timing offset |
| **Fleet visit order is sort-by-random-priority** | Classic assigns `Random(N)+1` to each fleet as a sort key (extraction: `(seed>>16) % N`), then processes in ascending key order. The Range `N` is dynamic per player. Exact replication requires the full PRNG call chain from validation, which is infeasible. **Rust uses deterministic slot order**, which produces byte-identical results against the oracle for all tested scenarios |
| **Combat reports emitted inline during weekly loop** | RESULTS.DAT writes happen inside the fleet pass. Do not defer all report generation to a post-simulation phase |
| **Combat triggered by first co-located hostile fleet** | the engine reads the opposing fleet, resolves combat, emits reports inline, then writes back. Opposing fleet's writeback happens later in the same pass |
| **Fleet destruction/capture dynamic** | destroyed fleets dropped from subsequent passes; captured fleets change ownership mid-simulation |
| **Pre-loop fleet setup phase** | fleet-battle has 5 pre-loop passes for captures/reassignments; non-combat scenarios skip this entirely |
| **Colonization is atomic on arrival** | ownership, armies (=1), name, status, production all set in one pass; economy starts the following tick |
| **Economy/autopilot gated by `player[0]` and runs after fleet loop** | only rogue mode (`0xFF`) empires get growth; economy reads post-combat fleet state |
| **File write ordering is stable** | FLEETS → RESULTS → DATABASE → PLAYER → PLANETS → CONQUEST → RANKINGS |
| **`00e8/024d` are yearly producer passes** | they mix state mutation and event production; some mutations are silent |

### What Is Still Open

| Open question | Current safe implementation posture |
| --- | --- |
| ~~exact PRNG shuffle algorithm~~ | **RESOLVED**: not a shuffle — sort-by-random-priority with dynamic Range. Exact replication infeasible. Slot order produces oracle-identical results |
| exact target-world aftermath predicates | keep aftermath behind world-state inspection, not hard-coded per-mission tables |
| production completion timing | avoid promising exact parity until more oracle evidence lands |

## Current Practical Step-4 Shape

The current best implementation shape for step `4` is:

```text
4a. Prepare transient simulation workspaces
4b. Annual movement update (one-time position advance for all fleets;
    store fractional travel state in tuple_c for multi-year journeys)
4c. Pre-loop fleet setup (captures/reassignments; skipped if none needed)
4d. Determine fleet visit order (PRNG shuffle seeded from game state)
4e. For each week 1..52 (EVENT SCHEDULING, not physics):
      For each fleet in visit order:
        - read fleet record
        - timing-window check: does this fleet have events to emit this week?
        - if co-located hostile: resolve combat + emit RESULTS.DAT inline
        - update weekly event state in fleet record
        - write fleet record
      Remove destroyed/captured fleets from active set
4f. Post-loop fleet scan (2 sequential reads of all fleet records)
4g. Economy/autopilot pass (rogue empires only; reads post-combat state)
4h. Producer/mutator passes on planet state (024d interior)
4i. DATABASE.DAT planet-specific updates
```

Key structural evidence:

- **movement is annual**: fleet positions update once per year, not
  per-week. Tuple_c (+0x19..+0x1E) stores Real48 fractional travel state
  for multi-year journeys (set during movement, cleared on arrival)
- **the 52-week loop is event scheduling**: stardates come from timing
  codes (+2/+7/+21/+30 week offsets), not from physical arrival time.
  A speed-3 fleet traveling 1 sector shows contact at week 50 (timing-code
  scheduled), not week 19 (physical arrival)
- non-combat fleet processing is exactly 4 I/O events per fleet per pass:
  seek, read, seek, write
- combat processing adds extra reads of opposing fleet(s) and inline
  RESULTS.DAT writes
- PLANETS.DAT is **never accessed** during the 52-pass fleet loop; planet
  economy/production changes happen after the fleet loop
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

That keeps the engine faithful to current evidence without claiming the final
oracle precedence is already fully solved.

## Target-World Aftermath Should Be State-Sensitive

Current practical evidence suggests that natural hostile-resolution aftermath on
the target world depends on more than the mission family.

In particular:

- two scenarios with different surrounding context but the same starting
  target-world payload can produce the same early target-world aftermath shape
- transplanting a different target-world seed into those same scenarios can
  change that shape materially
- but transplanting the stronger target-world seed into a bombard context does
  not recreate the same aftermath shape by itself

Implementation consequence:

- do not model target-world aftermath as:
  - `if mission == invade, write X`
  - `if mission == bombard, write Y`
- instead, prefer a rule shape closer to:
  - identify the hostile-resolution context
  - inspect the target-world state/class
  - choose the applicable aftermath update shape

Practical rule:

- neither hostile context alone nor target-world payload alone currently looks
  sufficient
- the engine should therefore choose target-world aftermath from the
  combination of:
  - hostile-resolution context
  - current target-world state/class

That keeps the Rust engine aligned with current oracle evidence while the exact
classic predicates are still being recovered.

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
  fleet_order = compute_fleet_visit_order(state)  // PRNG shuffle

  // Phase 4e: 52-week event scheduling loop (NOT physics sim)
  for week in 1..=52:
      for fleet in fleet_order.active_fleets():
          process_fleet_week(state, fleet, week, events)
          // inner body: read fleet, timing-window check,
          // combat if hostile, update weekly state, write fleet
      fleet_order.remove_destroyed_and_captured(state)

  // Phase 4f: post-loop fleet summary scan
  scan_all_fleets_for_summary(state, events)

  // Phase 4g: economy/autopilot (rogue empires only, post-combat)
  run_economy_autopilot(state)

  // Phase 4h: planet producer/mutator passes (024d interior)
  run_planet_producer_passes(state, work, events)

  // Phase 4i: database updates
  update_database_entries(state)

  // Phases 5-7
  canonicalize_events(events)
  emit_reports(state, events)
  rebuild_derived_outputs(state, events)
  flush_and_cleanup(directory, state, events)
```

Use that as a shape guide, not a frozen final ordering contract.

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
6. Keep step `4` subphases explicit and reorderable.
7. Treat mission-family timing as data/logic attached to the scheduler, not as
   one universal post-combat delay.
8. Prefer typed event records and explicit phase functions over giant
   cross-cutting mutation code.

## What This Document Does Not Claim

This document does not claim:

- ~~the exact PRNG shuffle algorithm for fleet visit order~~ (resolved: sort-by-random-priority, see ec-turn-cycle-spec.md section 4l)
- the exact producers for timing codes 7 and 8
- the exact target-world aftermath predicates
- production completion timing vs other subphases
- the semantic meaning of every still-raw planet/player field
- the original combat RNG or full Pascal-era implementation structure

It does claim:

- the major outer turn-cycle boundaries are strong enough to guide Rust
- **movement is annual** (one-time position update), not per-week
- **the 52-week fleet loop is event scheduling**, not physics simulation
- stardates come from timing codes (+2/+7/+21/+30), not physical arrival
- timing codes 3-6 are assigned by fleet composition; 1,2 from decoder
- mission resolution requires start-of-year position
- economy/autopilot is gated by `player[0]` and runs after the fleet loop
- colonization is atomic on arrival
- combat reports are emitted inline during the weekly loop
- producer/mutator passes are part of gameplay state mutation, not just
  report formatting
- event production, canonicalization, and report emission are distinct
  responsibilities and should stay distinct in Rust

## Relationship To Other Docs

- use [ec-turn-cycle-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-turn-cycle-spec.md)
  for oracle-backed phase evidence
- use [ec-timing-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-timing-spec.md)
  for weekly report/timing evidence
- use [approach.md](/home/mag/dev/esterian_conquest/docs/dev/approach.md)
  for project-level preservation and RE policy
- use [rust-architecture.md](/home/mag/dev/esterian_conquest/docs/dev/rust-architecture.md)
  for codebase structure and DOD rules
