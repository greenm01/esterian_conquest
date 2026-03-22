# ECMAINT Movement Audit

Compact movement-fidelity summary for the current controlled oracle work.

Status: solved enough for the current Rust engine. This doc records the
player-facing conclusions and the accepted compatibility seam; it is not a
standing requirement to keep chasing every hidden classic movement byte.

## What Is Settled

- movement is annual and happens before the weekly `1..52` scheduling loop
- the annual distance budget still follows the recovered `speed * 8 / 9`
  pattern
- classic does not simply resample from the rounded visible sector each year
- the controlled horizontal / diagonal / shallow / steep coordinate traces are
  now good enough to support the current Rust direct-stepper geometry
- persistent standing missions now have their own settled follow-up in
  [persistent-mission-oracle-audit.md](persistent-mission-oracle-audit.md)
- delayed hostile arrival behavior now has its own settled follow-up in
  [hostile-arrival-oracle-audit.md](hostile-arrival-oracle-audit.md)
- for Rust policy, unresolved hidden movement bytes are now treated as
  implementation detail unless they prove to change player-facing behavior or
  classic file safety

## What Changed In The Follow-Up

The remaining movement question is no longer just "which sector does the fleet
show up in next year?" The transit follow-up in
[transit-scratch-oracle-audit.md](transit-scratch-oracle-audit.md) established
two deeper constraints:

- classic leaves `0x19..0x1e` zero in the controlled transit turns checked so
  far, so Rust's current exact-position encoding there is an internal seam, not
  a recovered classic byte model
- one-shot movement completion is not keyed from the first rounded
  target-sector hit alone

Confirmed example:

- `MoveOnly`, `speed=3`, `10,10 -> 16,16`
- classic visible trace: `10,10 -> 11,11 -> 14,14 -> 16,16 -> 16,16`
- classic keeps `order=move`, `speed=3` on the first visible `16,16` tick
- classic clears to `hold`, `speed=0` on the following maintenance pass

That means the right current model is:

- the fleet has some hidden between-turn continuity state
- visible coordinates are rounded from that hidden state
- one-shot completion waits for the hidden path to be exhausted, not merely for
  the rounded visible sector to equal the target

## What Rust Now Does

Rust now avoids completing one-shot movement on the first rounded target-sector
hit. The movement stepper waits for the exact path endpoint before treating the
mission as complete, which fixes the confirmed `speed=3` diagonal `MoveOnly`
case.

This is not claimed as a full decode of classic hidden movement state. The
slower diagonal follow-up in
[transit-scratch-oracle-audit.md](transit-scratch-oracle-audit.md) is kept as
background evidence, but it is no longer treated as an active blocker for the
Rust engine by itself.

## Accepted Boundary

- preserve the recovered annual movement geometry and the confirmed diagonal
  completion delay that affect player-facing outcomes
- keep hazard-detour policy clearly separated from the classic direct-stepper
  geometry
- do not treat hidden movement scratch bytes as a fidelity target unless they
  are later shown to matter for gameplay or compatibility
