# Next Session

Use this as the restart brief. Historical detail belongs in
[next-session-archive.md](archive/next-session-archive.md), not here.

## Current State

- Architecture is stable:
  - `ec-data` = runtime/store/model
  - `ec-engine` = gameplay/rules API
  - `ec-compat` = classic `.DAT` import/export/oracle bridge
  - `ec-classic` = low-level classic record/codecs
- Phase 1 engine-boundary correction is now in place:
  - maintenance execution lives in `ec-engine/src/maint/`
  - shared maintenance event/result payloads live in
    `ec-data::maintenance_types`
  - `ec-data` no longer exports maintenance execution entrypoints
- Phase 2 boundary correction is now in place:
  - movement/pathfinding rule code lives in `ec-engine/src/navigation/`
  - raw fleet motion scratch-byte helpers live in
    `ec-data::fleet_motion_state`
- Phase 3 boundary correction is now in place:
  - setup/map-generation rule code lives in `ec-engine/src/setup/`
  - shared setup config parsing and baseline state builders remain in
    `ec-data`
- SQLite is the runtime source of truth for engine and TUI.
- Classic `.DAT` files are now an explicit compatibility edge, not live runtime
  state.
- Latest full baseline passed:
  - `cargo test -q`

## Current Goal

Validate and extend the classic-style fleet movement stepper beyond the current
controlled trace matrix.

The movement audit in [movement-oracle-audit.md](movement-oracle-audit.md)
established:

- movement is annual and happens before the weekly `1..52` report timeline
- the controlled horizontal / diagonal / shallow / steep `MoveOnly` probes now
  match the classic trace matrix
- Rust now keeps ETA on the same direct movement geometry
- `MoveOnly` is treated as complete on arrival:
  - fleet stops
  - speed becomes `0`
  - standing order becomes `Hold`
- `JoinAnotherFleet` and `RendezvousSector` now follow the manual-backed
  persistent-standing model:
  - join fleets keep chasing until they merge or the host is lost
  - rendezvous fleets wait at the assigned sector for later arrivals
  - rendezvous host selection is lowest fleet ID, not first arrival
- the persistent mission follow-up in
  [persistent-mission-oracle-audit.md](persistent-mission-oracle-audit.md)
  established:
  - `PatrolSector`, `GuardStarbase`, and `GuardBlockadeWorld` keep their order
    on arrival
  - those orders do **not** keep their travel speed; classic drops
    `current_speed` to `0` once the fleet reaches the target
  - `PatrolSector` and `GuardBlockadeWorld` settle into a rest-like tuple-c
    shape after arrival
  - `GuardStarbase` still has unresolved arrival scratch bytes and aux-state
    behavior

## Biggest Blockers

- The exact classic line-stepping / rounding rule is still inferred from oracle
  traces rather than decoded from source.
- The exact raw-byte meaning of the classic in-transit scratch fields is still
  not fully decoded.
- `GuardStarbase` still has partially unresolved scratch-byte / aux-state
  behavior during transit and after arrival.
- Hazard-driven detours still need broader oracle coverage so the classic direct
  stepper and the Rust pathfinding extension remain cleanly separated.

## Working Assumption

Treat a completed one-shot transit mission, including plain movement, as
complete on arrival:

- fleet stops
- speed becomes `0`
- standing order becomes `Hold`
- player must issue a new order to move again

Persistent standing orders and delayed hostile-world missions remain the
exceptions:

- `PatrolSector`, `GuardStarbase`, `GuardBlockadeWorld`,
  `JoinAnotherFleet`, and `RendezvousSector` stay armed after arrival or while
  waiting for their merge condition
- current classic coverage for patrol/guard orders shows that "stay armed"
  means the order persists, not the movement state: the fleet still stops and
  its `current_speed` becomes `0`
- `BombardWorld`, `InvadeWorld`, and `BlitzWorld` preserve their order through
  arrival so the ready mission resolves on the following tick

## Immediate Next Steps

1. Re-probe hostile-world arrivals to keep the delayed `BombardWorld` /
   `InvadeWorld` / `BlitzWorld` behavior aligned with the turn-cycle specs,
   especially whether those orders preserve speed or only preserve the order.
2. Decide whether the current Rust in-transit scratch encoding should be kept
   as a pragmatic compatibility layer or replaced with a more directly recovered
   classic byte model.
3. Deepen the `GuardStarbase` scratch-byte / aux-state audit now that the
   visible arrival semantics are settled:
   - why `mission_aux[0]` flips from `01` to `00`
   - why classic leaves a distinct nonzero `0x0d..0x12` payload on arrival
4. Re-run the classic oracle harness for the controlled movement matrix and
   confirm the raw movement scratch bytes are still acceptable.

## Structural Note

The three major gameplay subsystems now follow the intended split:

- `ec-engine` owns maintenance, movement/pathfinding, and setup/map-generation
  rule execution
- `ec-data` keeps runtime/store/model state plus shared config, builder, and
  raw record-layout helpers needed by those engine systems
