# Guard Starbase Runtime Audit

Focused `GuardStarbase` probes comparing Rust maintenance against classic `ECMAINT` and drilling into the remaining runtime-field mismatches.

Current takeaways from this probe set:

- Classic clears `mission_aux[0]` from `01` to `00` on the first maintenance pass for active `GuardStarbase` fleets, even while the mission remains armed.
- During the transit year, `GuardStarbase` uses the normal in-transit motion bytes in `0x0d..0x12`; the earlier doc note that classic left that window zero during transit was incorrect.
- In the controlled axial arrival cases below, classic converges on the same guarded-arrival `0x0d..0x12` payload after arrival: `7b/00/84/d8/89/1d`.
- If the guarded base is removed after arrival, classic abandons the mission on the following maintenance tick even though `mission_aux[0]` is already `00`; long-lived guard continuation is therefore keyed from the actual guarded base at the target, not from the original index byte.
- After the Rust runtime-linkage fix and guarded-arrival payload mirror, the only remaining mismatch in these controlled cases is the transit-year `0x1a..0x1e` window: classic leaves it zero while Rust still stores exact in-transit position there for geometry/ETA continuity.

| case | rust/classic match |
| --- | --- |
| guard-starbase-axial-a | no |
| guard-starbase-axial-b | no |

## guard-starbase-axial-a

- start: `8,8`
- target: `11,8`
- compare Rust: `yes`
- turn-by-turn match: `no`

| turn | Rust | Classic | Classic base state |
| ---: | --- | --- | --- |
| 0 | `8,8 order=guard_starbase spd=3 aux=[01, 01] 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `8,8 order=guard_starbase spd=3 aux=[01, 01] 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `player_starbases=1 base_count=1 base_active=1 base_id=1 base_coords=(11, 8)` |
| 1 | `10,8 order=guard_starbase spd=3 aux=[00, 01] 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/0a/00/08/42` | `10,8 order=guard_starbase spd=3 aux=[00, 01] 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/00/00/00/00` | `player_starbases=1 base_count=1 base_active=1 base_id=1 base_coords=(11, 8)` |
| 2 | `11,8 order=guard_starbase spd=0 aux=[00, 01] 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` | `11,8 order=guard_starbase spd=0 aux=[00, 01] 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` | `player_starbases=1 base_count=1 base_active=1 base_id=1 base_coords=(11, 8)` |
| 3 | `11,8 order=guard_starbase spd=0 aux=[00, 01] 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` | `11,8 order=guard_starbase spd=0 aux=[00, 01] 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` | `player_starbases=1 base_count=1 base_active=1 base_id=1 base_coords=(11, 8)` |

## guard-starbase-axial-b

- start: `11,10`
- target: `14,10`
- compare Rust: `yes`
- turn-by-turn match: `no`

| turn | Rust | Classic | Classic base state |
| ---: | --- | --- | --- |
| 0 | `11,10 order=guard_starbase spd=3 aux=[01, 01] 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `11,10 order=guard_starbase spd=3 aux=[01, 01] 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `player_starbases=1 base_count=1 base_active=1 base_id=1 base_coords=(14, 10)` |
| 1 | `13,10 order=guard_starbase spd=3 aux=[00, 01] 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/0d/00/0a/42` | `13,10 order=guard_starbase spd=3 aux=[00, 01] 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/00/00/00/00` | `player_starbases=1 base_count=1 base_active=1 base_id=1 base_coords=(14, 10)` |
| 2 | `14,10 order=guard_starbase spd=0 aux=[00, 01] 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` | `14,10 order=guard_starbase spd=0 aux=[00, 01] 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` | `player_starbases=1 base_count=1 base_active=1 base_id=1 base_coords=(14, 10)` |
| 3 | `14,10 order=guard_starbase spd=0 aux=[00, 01] 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` | `14,10 order=guard_starbase spd=0 aux=[00, 01] 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` | `player_starbases=1 base_count=1 base_active=1 base_id=1 base_coords=(14, 10)` |

## guard-starbase-base-lost-after-arrival

- start: `8,8`
- target: `11,8`
- compare Rust: `no`
- destroy guarded base after turn: `2`

| turn | Classic | Classic base state |
| ---: | --- | --- |
| 0 | `8,8 order=guard_starbase spd=3 aux=[01, 01] 0d..12=80/00/00/00/00/00 19..1e=81/00/00/00/00/00` | `player_starbases=1 base_count=1 base_active=1 base_id=1 base_coords=(11, 8)` |
| 1 | `10,8 order=guard_starbase spd=3 aux=[00, 01] 0d..12=7f/c0/fe/ff/ff/7f 19..1e=00/00/00/00/00/00` | `player_starbases=1 base_count=1 base_active=1 base_id=1 base_coords=(11, 8)` |
| 2 | `11,8 order=guard_starbase spd=0 aux=[00, 01] 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` | `player_starbases=1 base_count=1 base_active=1 base_id=1 base_coords=(11, 8)` |
| 3 | `11,8 order=hold spd=0 aux=[00, 01] 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00` | `player_starbases=0 base_count=0 base_active=N/A base_id=N/A base_coords=N/A` |

## Practical Rust Consequence

The runtime guard-starbase model should treat `mission_aux[0]` as an input/setup byte, not as the durable post-maint linkage key.
Rust should therefore:

- normalize the runtime `GuardStarbase` aux index byte to `00` during maintenance
- keep the mission armed while a friendly active base still exists at the guarded target coords
- abandon to `Hold` when that guarded base disappears, even if the aux index is already zero
- mirror the guarded-arrival `0x0d..0x12` payload as a compatibility shape even though its low-level meaning is still not decoded
- leave the transit-year `0x1a..0x1e` mismatch for the broader motion-scratch recovery pass

The controlled base-loss probe ended with classic trace:

`11,8 order=hold spd=0 aux=[00, 01] 0d..12=7b/00/84/d8/89/1d 19..1e=00/00/00/00/00/00`
