# ECMAINT Movement Audit

Controlled `MoveOnly` probes comparing classic `ECMAINT` against the Rust
movement stepper.

## What This Audit Establishes

- movement is annual and happens before the weekly `1..52` scheduling loop
- the yearly distance budget still follows the recovered `speed * 8 / 9`
  pattern
- classic does not resample from the rounded visible sector each year
- classic preserves in-transit fractional progress along the trip and only
  rounds when writing the visible position
- plain `MoveOnly` transit is complete on arrival:
  - fleet stops
  - speed becomes `0`
  - standing order becomes `Hold`

Rust now mirrors these controlled trace cases with focused regression tests in
`rust/ec-data/tests/movement_trace.rs`.

Persistent standing-order follow-up now lives in
[persistent-mission-oracle-audit.md](persistent-mission-oracle-audit.md). That
companion probe set establishes that `PatrolSector`, `GuardStarbase`, and
`GuardBlockadeWorld` keep their order on arrival, but they do **not** keep
their travel speed; classic stops them at the target and leaves a smaller,
more order-specific scratch-byte footprint than the generic old Rust arrival
stamp.

Delayed hostile-world follow-up now lives in
[hostile-arrival-oracle-audit.md](hostile-arrival-oracle-audit.md). That
companion probe set establishes the opposite arrival-state rule for
`BombardWorld`, `InvadeWorld`, and `BlitzWorld`: classic preserves both the
standing order and the current travel speed on the arrival tick, then resolves
the ready hostile mission on the following maintenance pass.

## Current Classic-Compatible Model

The best current model for classic movement is:

- keep the annual `speed * 8 / 9` travel budget
- persist the fleet's exact in-transit position between maintenance turns
- advance from that exact position toward the target
- round only when emitting the visible sector coordinates
- treat `MoveOnly` as complete when the visible rounded position reaches the
  target sector

This explains the diagonal and sloped traces much better than the older Rust
behavior that recomputed from the rounded visible sector each year.

## Trace Matrix

| case | speed | start | target | classic trace | classic arrival | Rust status |
| --- | ---: | --- | --- | --- | ---: | --- |
| `speed3-horizontal` | 3 | `10,10` | `16,10` | `10,10 -> 12,10 -> 15,10 -> 16,10` | 3 | matches |
| `speed3-diagonal` | 3 | `10,10` | `16,16` | `10,10 -> 11,11 -> 14,14 -> 16,16` | 3 | matches |
| `speed6-diagonal` | 6 | `10,10` | `16,16` | `10,10 -> 14,14 -> 16,16` | 2 | matches |
| `speed1-diagonal` | 1 | `10,10` | `13,13` | `10,10 -> 10,10 -> 11,11 -> 11,11 -> 12,12 -> 13,13` | 5 | matches |
| `speed3-shallow` | 3 | `10,10` | `16,12` | `10,10 -> 12,11 -> 15,12 -> 16,12` | 3 | matches |
| `speed3-steep` | 3 | `10,10` | `12,16` | `10,10 -> 11,12 -> 12,15 -> 12,16` | 3 | matches |

## Notes

- This audit is about movement geometry and arrival semantics, not the exact
  raw-byte encoding of classic in-transit scratch fields.
- The persistent mission follow-up confirms the same warning: visible movement
  semantics can be settled before every motion scratch byte is fully decoded.
- Threat-aware pathfinding remains a Rust policy layer. When visible hazards do
  not force a detour, the classic-compatible direct movement geometry above is
  the fidelity target.
