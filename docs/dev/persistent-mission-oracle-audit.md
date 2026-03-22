# ECMAINT Persistent Mission Audit

Controlled standing-mission probes comparing Rust maintenance against classic `ECMAINT`.

Current takeaways from this probe set:

- All three standing missions keep their order on arrival, but classic stops the fleet: `current_speed` becomes `0` instead of staying at the travel speed.
- `PatrolSector` and `GuardBlockadeWorld` both converge on the same classic post-arrival shape: order preserved, speed `0`, and tuple-c reset to `19=81 1a..1e=00`.
- `GuardStarbase` also stops on arrival, but its runtime normalization is more specialized than the other standing missions:
  - classic clears `mission_aux[0]` from `01` to `00` on the first maintenance pass and keeps it at `00`
  - classic uses normal in-transit motion bytes in `0x0d..0x12` during the travel year
  - classic converges on a distinct guarded-arrival `0x0d..0x12` payload after arrival in the controlled cases
  - if the guarded base later disappears, classic abandons to `Hold` even though `mission_aux[0]` is already `00`
- After the Rust standing-arrival and guard-runtime fixes, the remaining mismatches in this audit are scratch-byte details rather than visible mission semantics.

Scope of this probe set:

- `PatrolSector`
- `GuardStarbase`
- `GuardBlockadeWorld`
- controlled axial `speed=3` arrivals with one post-arrival maintenance tick

The goal is not full mission-combat semantics. It is to pin down:

- whether classic preserves the standing order after arrival
- whether classic preserves the fleet speed after arrival
- what classic writes into the `0x19..0x1e` arrival-state scratch window
- whether the next maintenance tick still treats the fleet as armed on that order

| case | rust arrival | classic arrival | arrival byte match | turn-by-turn match |
| --- | ---: | ---: | --- | --- |
| patrol-speed3-axial | 2 | 2 | yes | no |
| guard-starbase-speed3-axial | 2 | 2 | yes | no |
| guard-blockade-speed3-axial | 2 | 2 | yes | no |

## patrol-speed3-axial

- order code: `3`
- speed: `3`
- start: `8,10`
- target: `11,10`
- initial Rust ETA: `2`
- Rust arrival turn: `2`
- Classic arrival turn: `2`
- Rust arrival bytes: `19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic arrival bytes: `19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Rust post-arrival trace: `11,10 order=patrol spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic post-arrival trace: `11,10 order=patrol spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- turn-by-turn match: `no`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `8,10 order=patrol spd=3 aux=[01, 00] eta=2 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `8,10 order=patrol spd=3 aux=[01, 00] eta=2 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 1 | `10,10 order=patrol spd=3 aux=[01, 00] eta=1 19=00 1a=00 1b=0a 1c=00 1d=0a 1e=42` | `10,10 order=patrol spd=3 aux=[01, 00] eta=1 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 2 | `11,10 order=patrol spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `11,10 order=patrol spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 3 | `11,10 order=patrol spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `11,10 order=patrol spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |

## guard-starbase-speed3-axial

- order code: `4`
- speed: `3`
- start: `8,8`
- target: `11,8`
- initial Rust ETA: `None`
- Rust arrival turn: `2`
- Classic arrival turn: `2`
- Rust arrival bytes: `19=00 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic arrival bytes: `19=00 1a=00 1b=00 1c=00 1d=00 1e=00`
- Runtime follow-up:
  [guard-starbase-runtime-audit.md](guard-starbase-runtime-audit.md)
- Rust post-arrival trace: `11,8 order=guard_starbase spd=0 aux=[00, 01] eta=arrived 19=00 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic post-arrival trace: `11,8 order=guard_starbase spd=0 aux=[00, 01] eta=arrived 19=00 1a=00 1b=00 1c=00 1d=00 1e=00`
- turn-by-turn match: `no`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `8,8 order=guard_starbase spd=3 aux=[01, 01] eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `8,8 order=guard_starbase spd=3 aux=[01, 01] eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 1 | `10,8 order=guard_starbase spd=3 aux=[00, 01] eta=unreachable 19=00 1a=00 1b=0a 1c=00 1d=08 1e=42` | `10,8 order=guard_starbase spd=3 aux=[00, 01] eta=unreachable 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 2 | `11,8 order=guard_starbase spd=0 aux=[00, 01] eta=arrived 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` | `11,8 order=guard_starbase spd=0 aux=[00, 01] eta=arrived 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 3 | `11,8 order=guard_starbase spd=0 aux=[00, 01] eta=arrived 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` | `11,8 order=guard_starbase spd=0 aux=[00, 01] eta=arrived 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` |

## guard-blockade-speed3-axial

- order code: `5`
- speed: `3`
- start: `8,8`
- target: `11,8`
- initial Rust ETA: `None`
- Rust arrival turn: `2`
- Classic arrival turn: `2`
- Rust arrival bytes: `19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic arrival bytes: `19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Rust post-arrival trace: `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic post-arrival trace: `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- turn-by-turn match: `no`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `8,8 order=guard_blockade spd=3 aux=[01, 00] eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `8,8 order=guard_blockade spd=3 aux=[01, 00] eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 1 | `10,8 order=guard_blockade spd=3 aux=[01, 00] eta=unreachable 19=00 1a=00 1b=0a 1c=00 1d=08 1e=42` | `10,8 order=guard_blockade spd=3 aux=[01, 00] eta=unreachable 19=00 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 2 | `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 3 | `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `11,8 order=guard_blockade spd=0 aux=[01, 00] eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
