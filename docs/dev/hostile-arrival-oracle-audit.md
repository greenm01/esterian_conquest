# ECMAINT Hostile Arrival Audit

Controlled delayed hostile-world probes comparing Rust maintenance against classic `ECMAINT`.

Current takeaways from this probe set:

- `BombardWorld`, `InvadeWorld`, and `BlitzWorld` all preserve both the standing order and the current travel speed on the arrival tick.
- On arrival, all three stamp the same ready hostile tuple-c payload: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`.
- The ready assault/bombardment does not resolve on the travel tick; it resolves on the following maintenance tick if the fleet is still valid for that mission.
- In the controlled strong-composition probe, `BlitzWorld` matches the same delayed shape as `BombardWorld` and `InvadeWorld`; the earlier weak blitz probe was a bad setup, not a different rule.

Scope of this probe set:

- one-sector delayed hostile arrivals
- `BombardWorld`, `InvadeWorld`, and `BlitzWorld`
- one follow-up ready-resolution tick after arrival
- `BlitzWorld` uses a strong combined-arms fleet so the second tick exercises a valid assault path

| case | rust arrival | classic arrival | arrival byte match | turn-by-turn match |
| --- | ---: | ---: | --- | --- |
| bombard-delayed | 1 | 1 | yes | yes |
| invade-delayed | 1 | 1 | yes | yes |
| blitz-delayed-strong | 1 | 1 | yes | yes |

## bombard-delayed

- source scenario: `bombard`
- fleet record: `3`
- Rust arrival turn: `1`
- Classic arrival turn: `1`
- Rust arrival bytes: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
- Classic arrival bytes: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
- Rust resolution trace: `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic resolution trace: `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `16,13 order=bombard spd=3 eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `16,13 order=bombard spd=3 eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 1 | `15,13 order=bombard spd=3 eta=arrived 19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f` | `15,13 order=bombard spd=3 eta=arrived 19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f` |
| 2 | `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |

## invade-delayed

- source scenario: `invade`
- fleet record: `3`
- Rust arrival turn: `1`
- Classic arrival turn: `1`
- Rust arrival bytes: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
- Classic arrival bytes: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
- Rust resolution trace: `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic resolution trace: `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `16,13 order=invade spd=3 eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `16,13 order=invade spd=3 eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 1 | `15,13 order=invade spd=3 eta=arrived 19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f` | `15,13 order=invade spd=3 eta=arrived 19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f` |
| 2 | `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |

## blitz-delayed-strong

- source scenario: `invade`
- fleet record: `3`
- Rust arrival turn: `1`
- Classic arrival turn: `1`
- Rust arrival bytes: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
- Classic arrival bytes: `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`
- Rust resolution trace: `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- Classic resolution trace: `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00`
- turn-by-turn match: `yes`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `16,13 order=blitz spd=3 eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `16,13 order=blitz spd=3 eta=unreachable 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
| 1 | `15,13 order=blitz spd=3 eta=arrived 19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f` | `15,13 order=blitz spd=3 eta=arrived 19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f` |
| 2 | `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` | `15,13 order=hold spd=0 eta=arrived 19=81 1a=00 1b=00 1c=00 1d=00 1e=00` |
