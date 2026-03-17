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
| - movement                   |
| - hostile contact / combat   |
| - delayed mission effects    |
| - producer/mutator passes    |
| - durable event creation     |
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
Run yearly simulation core
  |
  +--> mutate durable state
  +--> stage durable summary/events
  +--> produce some silent derived changes
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
+------------------------------+
    |
    +--> movement phase
    |
    +--> contact / combat / mission consequences
    |
    +--> producer/mutator passes
    |      |
    |      +--> durable state mutation
    |      +--> durable event creation
    |
    +--> internal weekly timing assignments
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

### What Is Still Open

| Open question | Current safe implementation posture |
| --- | --- |
| economy vs movement ordering | keep these as distinct subphases with explicit orchestration, not hard-coded hidden assumptions |
| production completion timing | avoid promising exact parity until more oracle evidence lands |
| command normalization timing | keep order sanitation/prep separate from outcome emission |
| exact combat-placement relative to producer passes | keep combat/outcome handling and producer passes separable in the driver |
| mission-family-specific aftermath timing | allow different mission families to schedule follow-on effects differently |
| exact overwrite precedence when two subphases touch the same target-world fields | centralize world-state writes in ordered subphase functions; do not hide them behind unordered helper side effects |
| exact target-world-state predicates that choose one aftermath shape over another | keep aftermath shaping behind explicit world-state inspection, not hard-coded per-mission tables alone |
| exact semantic meaning of late timing code classes | keep event timing classification explicit and typed in Rust, but defer final code-to-semantics mapping until oracle evidence lands |

## Current Practical Step-4 Shape

The current best implementation shape for step `4` is:

```text
4a. Prepare transient simulation workspaces
4b. Run explicit movement phase
4c. Resolve contact/combat and immediate redirects/retreats
4d. Run yearly producer/mutator passes
4e. Schedule and/or apply delayed mission consequences
4f. Finish durable event creation for later canonicalization
```

Important constraint:

- this is a practical Rust shape, not a claim that the oracle executes these
  in exactly this final order in every case
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

This is the current recommended engine shape for `rust-maint`.

```text
run_turn(directory):
  gate_result = run_gate_and_recovery(directory)
  if gate_result.skip:
      return

  state = load_and_validate(directory)

  work = create_turn_workspaces(state)
  events = create_event_pool()

  run_movement_phase(state, work, events)
  run_contact_and_combat_phase(state, work, events)
  run_producer_passes(state, work, events)
  run_delayed_mission_phase(state, work, events)

  canonicalize_events(events)
  emit_weekly_reports(state, events)
  rebuild_derived_outputs(state, events)
  flush_and_cleanup(directory, state, events)
```

Use that as a shape guide, not a frozen final ordering contract.

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

- the exact final oracle order of every step-`4` subphase
- the semantic meaning of every still-raw planet/player field
- the original combat RNG or full Pascal-era implementation structure

It does claim:

- the major outer turn-cycle boundaries are now strong enough to guide Rust
- step `4` should be implemented as a structured multi-subphase engine
- producer/mutator passes are part of gameplay state mutation, not just report
  formatting
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
