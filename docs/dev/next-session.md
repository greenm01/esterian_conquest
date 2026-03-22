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

Mirror classic fleet movement and ETA more closely.

The movement audit in [movement-oracle-audit.md](movement-oracle-audit.md)
established:

- movement is annual and happens before the weekly `1..52` report timeline
- axial travel is directionally right
- diagonal and sloped travel still diverges from classic
- classic likely preserves fractional progress along a fixed launch-to-target
  line, consistent with Turbo Pascal-style integer/fixed-point stepping
- `MoveOnly` arrival cleanup still needs to be settled and mirrored

## Biggest Blockers

- The exact classic line-stepping / rounding rule is still inferred from oracle
  traces rather than decoded from source.
- `MoveOnly` arrival behavior is not fully settled:
  - several probes clear to `Hold` with `speed=0`
  - one diagonal `speed=3` case still showed `MoveOnly` surviving at arrival
- ETA must follow the same classic stepping rule as movement.

## Working Assumption

Until contradicted by better oracle evidence, treat a completed mission,
including plain movement, as complete on arrival:

- fleet stops
- speed becomes `0`
- standing order becomes `Hold`
- player must issue a new order to move again

This matches user intent and most current oracle arrival probes. Re-test the
one conflicting `MoveOnly` diagonal case before locking the rule in.

## Immediate Next Steps

1. Replace the current movement position update with a classic-style persistent
   line-progress stepper.
2. Re-run the movement audit matrix and compare turn-by-turn traces.
3. Re-probe `MoveOnly` arrival, especially the diagonal `speed=3` case, and
   settle whether arrival always clears to `Hold`.
4. Once movement matches, keep the shared ETA helper on the same stepping rule.

## Structural Note

Pathfinding/movement helpers still live with the active maintenance/runtime
implementation. If that code moves later, move the whole movement stack across
the crate boundary together rather than creating an `ec-data -> ec-engine`
cycle.
