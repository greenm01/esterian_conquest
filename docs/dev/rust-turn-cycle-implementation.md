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
- which parts of step `4` are still provisional

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
- pretending step `4` is fully recovered when it is not

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

Step `4` is the only major unresolved block. The right Rust posture is:

- implement it as a structured sequence of subphases
- mark some subphases as provisional
- avoid baking in one final canonical order until the oracle evidence settles

### Step 4 Block Diagram

```text
Validated Durable State
    |
    v
+------------------------------+
| Step 4 Simulation Driver     |
| (weekly fleet-processing     |
|  loop, 52 iterations)        |
+------------------------------+
    |
    +--[for week 1..52]------+
    |                         |
    |  +--> fleet activation  |
    |  |    (data-dependent   |
    |  |     visit order)     |
    |  |                      |
    |  +--> movement          |
    |  |                      |
    |  +--> contact / combat  |
    |  |    / mission resolve |
    |  |                      |
    |  +--> inline report     |
    |  |    emission          |
    |  |    (RESULTS.DAT      |
    |  |     writes happen    |
    |  |     mid-loop)        |
    |  |                      |
    |  +--> producer/mutator  |
    |       passes            |
    |       (state mutation + |
    |        event creation)  |
    |                         |
    +-------------------------+
    |
    v
Updated Durable State + Durable Event Pool
```

### What Is Settled

| Point | Practical meaning for Rust |
| --- | --- |
| Movement is a real named engine boundary | keep movement as an explicit subphase, not a side effect hidden inside reporting |
| Delayed missions exist | do not treat arrival, bombardment, invasion, and similar outcomes as one atomic same-step event family |
| Internal weekly timing exists | event generation and report emission need a weekly scheduler model, not one end-of-year dump |
| Late weekly placement uses explicit timing-window logic | keep report scheduling as a real scheduler stage with computed windows and accept/reject tests, not a flat per-event offset table |
| `00e8/024d` are yearly producer passes | keep room for dedicated producer/mutator subphases inside step `4` |
| `024d` mixes state mutation and event production | do not force a false boundary where all state mutation finishes before any durable event creation starts |
| Some producer-side world mutation is silent | do not assume every important step-4 change creates a report/message immediately |
| Some neighboring step-4 subphases appear to write overlapping target-world state | do not assume one clean owner per world field; the driver needs ordered overwrite behavior and explicit subphase boundaries |
| Some natural hostile-resolution target-world consequences depend on the starting world payload/class | do not key target-world aftermath only by mission family; keep room for world-state-sensitive aftermath rules |
| **The yearly simulation is a 52-iteration weekly fleet-processing loop** | the Rust driver should model step 4 as a `for week in 1..=52` loop over the fleet table, not as separate movement/combat/producer macro-phases |
| **Fleet visit order is PRNG-shuffled per game state** | do not iterate fleets in slot order; the engine uses a PRNG-seeded shuffle (seeded from planet data) that produces different orderings per game state. For initial Rust, use a deterministic order; exact PRNG replication requires static RE of the Borland Pascal Random function |
| **Combat reports are emitted inline during the weekly loop** | RESULTS.DAT writes happen inside the fleet pass (observed at pass 7 in fleet-battle). Do not defer all report generation to a post-simulation phase |
| **Combat resolution is triggered by first co-located hostile fleet** | when a fleet is processed and encounters a hostile fleet at the same location, the engine reads the opposing fleet, resolves combat, emits reports inline, then writes back the processing fleet. The opposing fleet's writeback happens later in the same pass |
| **Fleet destruction reduces the active fleet set mid-simulation** | the weekly loop must handle fleet removal during iteration; destroyed fleets are dropped from subsequent passes |
| **Fleet slot reassignment (capture) can change fleet ownership** | fleet slots can change empire ownership during the simulation; reassigned slots may be excluded from the weekly visit set |
| **File write ordering is stable**: FLEETS first, then RESULTS (in combat), then DATABASE, PLAYER, PLANETS, CONQUEST, RANKINGS | keep the Rust flush phase in this order for oracle parity |
| **Movement is position-first, mission-resolution-next-year** | a fleet that arrives at its target during the 52-week loop updates its position, but the mission (bombard, colonize, etc.) resolves only the following year — the fleet must be at its target at the start of the year for resolution. Co-located fleets resolve within the same tick |
| **Colonization is atomic on arrival** | when a colonize fleet resolves, ownership, armies (=1), name, status, and potential production are all set in one pass; economy starts the following tick |
| **Economy/autopilot processing gated by `player[0]`** | only empires with `player[0] = 0xFF` (rogue mode) get economy/army/battery growth; civil disorder empires (`player[0] = 0x00`) are frozen. `player[0x6D]` (autopilot flag) drives army/battery building within the rogue pass |
| **Economy/autopilot runs after the fleet loop** | PLANETS.DAT is never accessed during the 52-pass fleet loop (file-I/O evidence); economy outcomes depend on post-combat fleet state (with/without combat comparison shows different army growth). Keep economy as a post-fleet-loop pass in Rust |
| **Pre-loop fleet setup phase exists for captures/reassignments** | fleet-battle has 5 pre-loop fleet write passes before the 52-week loop; non-combat scenarios skip it entirely (0 pre-loop passes, exactly 52 total passes). Model as a distinct pre-loop subphase |

### What Is Still Open

| Open question | Current safe implementation posture |
| --- | --- |
| exact PRNG for fleet visit order shuffle | use a deterministic order for now; exact parity requires reverse-engineering the Borland Pascal Random seed from game state |
| production completion timing | avoid promising exact parity until more oracle evidence lands |
| exact inner-loop body structure | the per-fleet-per-week body does: read → combat check → report emit → write; but the exact placement of movement decrement, order execution, and producer passes within that body is not fully settled |
| mission-family-specific aftermath timing | allow different mission families to schedule follow-on effects differently |
| exact target-world-state predicates that choose one aftermath shape over another | keep aftermath shaping behind explicit world-state inspection, not hard-coded per-mission tables alone |
| pre-loop fleet setup phase | fleet-battle has 5 pre-loop fleet write passes (captures/reassignments) before the 52-week loop; non-combat scenarios skip this entirely. Model as a distinct pre-loop subphase in Rust |

## Current Practical Step-4 Shape

The current best implementation shape for step `4` is:

```text
4a. Prepare transient simulation workspaces
4b. Determine fleet visit order (PRNG shuffle seeded from game state)
4c. For each week 1..52:
      For each fleet in visit order:
        - read fleet record
        - if co-located hostile fleet: read opposing fleet, resolve combat,
          emit RESULTS.DAT reports inline
        - update fleet state (movement decrement, order execution, etc.)
        - write fleet record
      Remove destroyed/captured fleets from active set
4d. Post-loop fleet scan (2 sequential reads of all fleet records)
4e. Producer/mutator passes on planet state (024d interior)
4f. DATABASE.DAT planet-specific updates
```

Key structural evidence:

- non-combat fleet processing is exactly 4 I/O events per fleet per pass:
  seek, read, seek, write (confirmed: 4.0 events/write in bombard, econ,
  planet-build, invade)
- combat processing adds extra reads of opposing fleet(s) and inline
  RESULTS.DAT writes (confirmed: 4.2 events/write in fleet-battle)
- PLANETS.DAT is **never accessed** during the 52-pass fleet loop; planet
  economy/production changes happen after the fleet loop, not during it
- after the fleet loop, 2 sequential reads of all fleet records occur
  (post-loop summary scan)
- DATABASE.DAT planet-specific writes follow the summary scan
- the flush order (PLAYER → PLANETS → CONQUEST → RANKINGS) follows last

Important constraint:

- this is a practical Rust shape informed by file-I/O trace evidence
  showing exactly 52 fleet write passes with inline RESULTS.DAT emission
- the driver should therefore make these boundaries explicit enough to reorder
  later if new oracle evidence demands it
- it should also allow later subphases to overwrite some earlier world-state
  changes on the same target, because current probes suggest that can happen in
  at least some step-`4` families

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
  fleet_order = compute_fleet_visit_order(state)  // PRNG shuffle

  // Phase 4c: 52-week fleet processing loop
  for week in 1..=52:
      for fleet in fleet_order.active_fleets():
          process_fleet_week(state, fleet, week, events)
          // inner body: read fleet, check co-located hostiles,
          // resolve combat + emit RESULTS inline, update fleet, write fleet
      fleet_order.remove_destroyed_and_captured(state)

  // Phase 4d: post-loop fleet summary scan (2 sequential reads)
  scan_all_fleets_for_summary(state, events)

  // Phase 4e: planet producer/mutator passes (024d interior)
  run_planet_producer_passes(state, work, events)

  // Phase 4f: database updates
  update_database_entries(state)

  // Phases 5-7
  canonicalize_events(events)
  emit_reports(state, events)
  rebuild_derived_outputs(state, events)
  flush_and_cleanup(directory, state, events)
```

Use that as a shape guide, not a frozen final ordering contract.

Key structural evidence:

- the 52-week fleet loop is the simulation core; each fleet gets exactly
  1 read + 1 write per pass (4 I/O events) in non-combat weeks
- combat adds inline reads of opposing fleets + RESULTS.DAT writes
- PLANETS.DAT is never accessed during the fleet loop; economy/production
  changes happen after the fleet loop in the producer passes
- after the fleet loop, 2 sequential scans of all fleet records occur
  (post-loop summary building)
- DATABASE.DAT planet-specific writes follow the summary scan
- the flush phase writes PLAYER, PLANETS, CONQUEST, RANKINGS in that order

Practical refinement:

- if a subphase writes target-world state that another subphase may also touch,
  keep that write path visible in the driver-level ordering
- do not bury those writes in unrelated report builders or broad cleanup code

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

- the exact PRNG for fleet visit order shuffle
- the exact inner per-fleet-per-week body structure (movement decrement vs
  order execution vs producer pass placement)
- the exact trigger for fleet incremental activation in early passes
- mission-family-specific aftermath timing constants
- the semantic meaning of every still-raw planet/player field
- the original combat RNG or full Pascal-era implementation structure

It does claim:

- the major outer turn-cycle boundaries are strong enough to guide Rust
- step `4` is a 52-week fleet-processing loop, not separate macro-phases
- movement is position-first with next-year mission resolution
- economy/autopilot processing is gated by `player[0]` and runs outside
  the fleet loop
- colonization is atomic on arrival
- combat reports are emitted inline during the fleet loop
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
