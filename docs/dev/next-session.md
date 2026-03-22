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

Keep the Rust engine moving forward on player-facing and compatibility-relevant
work without freezing on hidden classic implementation quirks.

Movement is now considered solved enough for the current Rust engine.

The movement audit in [movement-oracle-audit.md](movement-oracle-audit.md),
plus the transit follow-up in
[transit-scratch-oracle-audit.md](transit-scratch-oracle-audit.md),
established:

- movement is annual and happens before the weekly `1..52` report timeline
- the controlled horizontal / diagonal / shallow / steep `MoveOnly` probes now
  match the classic coordinate trace matrix closely enough for the current
  direct-stepper geometry
- Rust now keeps ETA on the same direct movement geometry for the currently
  covered cases
- classic leaves `0x19..0x1e` zero in the controlled transit turns checked so
  far, so Rust's current exact-position encoding there is still a pragmatic
  internal seam rather than a recovered classic byte model
- one-shot movement completion is not keyed from the first rounded
  target-sector hit alone
- the confirmed `MoveOnly speed=3 diagonal` case can show the fleet in the
  target sector for one maintenance tick before the move actually clears to
  `Hold`
- Rust now avoids completing one-shot movement on the first rounded
  target-sector hit, which fixes that confirmed diagonal case
- unresolved hidden movement bytes and low-signal completion quirks are no
  longer treated as active blockers by themselves
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

- Economy semantics are no longer blocked on the starbase `5x` question:
  - the manuals explicitly tie `5x` to build capacity
  - the current Ghidra pass did not support a starbase `5x` growth claim
  - the new black-box follow-up in
    [starbase-economy-oracle-audit.md](starbase-economy-oracle-audit.md)
    reconfirmed the commissioned-starbase / `5x` build-capacity side
  - the exact classic starbase growth bonus and tax-burden formula are still
    unrecovered because the current generated colony probe remains too noisy to
    trust semantically
- The exact weekly `1..52` assignment and dated-report process inside
  maintenance still deserve continued recovery when that work is directly
  useful.
- Hazard-driven detours still need broader coverage so the classic direct
  stepper and the Rust pathfinding extension stay conceptually separate, but
  this is no longer a movement blocker.

## Working Assumption

Treat the current Rust movement model as good enough when it preserves the
player-facing travel pattern and mission behavior that matter. Hidden classic
scratch bytes and ambiguous low-level completion artifacts are documented, but
they are not a required fidelity target by themselves.

For one-shot transit missions, including plain movement:

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

1. Keep the starbase `5x growth` question closed unless stronger new oracle or
   Ghidra evidence appears. Current evidence supports `5x` build capacity, not
   `5x` growth.
2. If the starbase economy thread is revisited, do **not** reuse the current
   generated colony sweep as proof of an exact classic formula:
   - first recover a cleaner accepted planet-state oracle baseline
   - or do a deeper static RE pass on the planet-side economy functions in the
     unwrapped `ECMAINTU.EXE` project
3. Verify in Ghidra whether any explicit classic starbase/tax branch supports
   the manuals' "`67%` to `70%`" tolerance language.
4. Keep movement closed unless a concrete player-facing or compatibility issue
   appears.
5. Continue the weekly maintenance/report recovery work where it materially
   improves the Rust engine or report timing.

## Structural Note

The three major gameplay subsystems now follow the intended split:

- `ec-engine` owns maintenance, movement/pathfinding, and setup/map-generation
  rule execution
- `ec-data` keeps runtime/store/model state plus shared config, builder, and
  raw record-layout helpers needed by those engine systems
