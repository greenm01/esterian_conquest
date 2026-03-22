# Next Session

Use this as the restart brief. Historical detail belongs in
[next-session-archive.md](archive/next-session-archive.md), not here.

## Current State

- Architecture is stable:
  - `ec-data` = runtime/store/model
  - `ec-engine` = gameplay/maintenance API
  - `ec-compat` = classic `.DAT` import/export/oracle bridge
  - `ec-classic` = low-level classic record/codecs
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

## Biggest Blockers

- The exact classic line-stepping / rounding rule is still inferred from oracle
  traces rather than decoded from source.
- The exact raw-byte meaning of the classic in-transit scratch fields is still
  not fully decoded.
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
- `BombardWorld`, `InvadeWorld`, and `BlitzWorld` preserve their order through
  arrival so the ready mission resolves on the following tick

## Immediate Next Steps

1. Re-run the classic oracle harness for the controlled movement matrix and
   confirm the raw movement scratch bytes are still acceptable.
2. Extend the same movement tracing approach to persistent missions such as
   `PatrolSector`, `GuardStarbase`, and `GuardBlockadeWorld`.
3. Re-probe hostile-world arrivals to keep the delayed `BombardWorld` /
   `InvadeWorld` / `BlitzWorld` behavior aligned with the turn-cycle specs.
4. Decide whether the current Rust in-transit scratch encoding should be kept as
   a pragmatic compatibility layer or replaced with a more directly recovered
   classic byte model.

## Structural Note

Pathfinding/movement helpers still live with the active maintenance/runtime
implementation. If that code moves later, move the whole movement stack across
the crate boundary together rather than creating an `ec-data -> ec-engine`
cycle.
