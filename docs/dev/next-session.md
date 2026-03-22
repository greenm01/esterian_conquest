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
  - the `GuardStarbase` runtime follow-up in
    [guard-starbase-runtime-audit.md](guard-starbase-runtime-audit.md)
    established:
    - classic clears `mission_aux[0]` from `01` to `00` on the first
      maintenance pass and keeps it at `00`
    - classic still keeps the mission armed while the guarded base exists at
      the target coords
    - if that guarded base disappears later, classic abandons to `Hold` even
      with `mission_aux[0] = 00`
    - Rust now mirrors the controlled guarded-arrival `0x0d..0x12` payload;
      the remaining mismatch is the transit-year `0x1a..0x1e` encoding
- the delayed hostile follow-up in
  [hostile-arrival-oracle-audit.md](hostile-arrival-oracle-audit.md)
  established:
  - `BombardWorld`, `InvadeWorld`, and `BlitzWorld` all preserve both order
    and `current_speed` on the arrival tick
  - all three stamp the same ready hostile tuple-c payload on arrival:
    `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
  - the hostile-world step still resolves on the following maintenance tick,
    not on the travel tick

## Biggest Blockers

- The exact classic line-stepping / rounding rule is still inferred from oracle
  traces rather than decoded from source.
- The exact raw-byte meaning of the classic in-transit scratch fields is still
  not fully decoded.
- `GuardStarbase` still has partially unresolved raw movement scratch bytes:
  - why classic leaves `0x1a..0x1e = 00` during the transit year where Rust
    currently stores exact in-transit position
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
- `BombardWorld`, `InvadeWorld`, and `BlitzWorld` preserve both their order
  and their current travel speed through arrival so the ready mission resolves
  on the following tick

## Immediate Next Steps

1. Decide whether the current Rust in-transit scratch encoding should be kept
   as a pragmatic compatibility layer or replaced with a more directly recovered
   classic byte model.
2. Deepen the remaining `GuardStarbase` scratch-byte audit now that the aux
   normalization rule is settled:
   - why classic leaves `0x1a..0x1e = 00` during the transit year
3. Re-run the classic oracle harness for the controlled movement matrix and
   confirm the raw movement scratch bytes are still acceptable.
4. Broaden hazard-detour oracle coverage so the classic direct stepper and the
   Rust visible-hazard routing extension stay cleanly separated.

## Structural Note

The three major gameplay subsystems now follow the intended split:

- `ec-engine` owns maintenance, movement/pathfinding, and setup/map-generation
  rule execution
- `ec-data` keeps runtime/store/model state plus shared config, builder, and
  raw record-layout helpers needed by those engine systems
