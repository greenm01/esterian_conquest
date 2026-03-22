# ECMAINT Movement Audit

Controlled `MoveOnly` probes comparing Rust maintenance against classic `ECMAINT`.

Current takeaways from this probe set:

- Annual movement is confirmed to happen before the weekly `1..52` scheduling loop; the probe directories only advanced once per maintenance turn.
- Horizontal `speed=3` travel matches classic turn-by-turn until arrival, so the current Rust `speed * 8 / 9` annual distance budget is at least directionally correct on axial paths.
- Diagonal and sloped routes do **not** match classic intermediate positions. Classic advances more conservatively than the current Rust straight-line rounding path.
- The clearest ETA miss is the `speed=1` diagonal case: Rust predicts and arrives in `4` years, while classic arrives in `5`.
- Classic clears `MoveOnly` to `hold` with `speed=0` on arrival in the horizontal, `speed=6` diagonal, shallow, and steep probes. Rust currently preserves `MoveOnly` on arrival.
- This also makes early contact more plausible than it first looked: the current 4-player mapgen target is an `18x18` map with roughly `5` sectors of minimum homeworld spacing, while classic `speed=3` probes still reach `6` sectors of axial/diagonal separation in `3` maintenance turns.

## Likely Classic Movement Model

The classic traces suggest that `ECMAINT` is not simply recomputing a fresh
straight-line move from the fleet's rounded sector every year.

The more likely model is:

- keep a cumulative annual travel budget using the already observed `speed`
  scaling
- preserve fractional progress along the original launch-to-target line
- snap that progress back to sector coordinates only when writing the visible
  fleet position

This fits the diagonal traces much better than a naive per-turn resample. In
particular:

- `speed=3`, `(10,10) -> (16,16)` advances `10,10 -> 11,11 -> 14,14 -> 16,16`
- `speed=1`, `(10,10) -> (13,13)` advances
  `10,10 -> 10,10 -> 11,11 -> 11,11 -> 12,12 -> 13,13`

Given that classic EC was built with Turbo Pascal-era types, the safest current
guess is that the game is using integer or fixed-point stepping with persistent
fractional progress, not floating-point-style re-evaluation each turn.

This remains an inference from the oracle traces, not a decoded source-level
fact.

| case | speed | start | target | rust ETA | rust arrival | classic arrival | trace match |
| --- | ---: | --- | --- | ---: | ---: | ---: | --- |
| speed3-horizontal | 3 | `10,10` | `16,10` | 3 | 3 | 3 | no |
| speed3-diagonal | 3 | `10,10` | `16,16` | 3 | 3 | 3 | no |
| speed6-diagonal | 6 | `10,10` | `16,16` | 2 | 2 | 2 | no |
| speed1-diagonal | 1 | `10,10` | `13,13` | 4 | 4 | 5 | no |
| speed3-shallow | 3 | `10,10` | `16,12` | 3 | 3 | 3 | no |
| speed3-steep | 3 | `10,10` | `12,16` | 3 | 3 | 3 | no |

## speed3-horizontal

- speed: `3`
- start: `10,10`
- target: `16,10`
- initial Rust ETA: `3`
- Rust arrival turn: `3`
- Classic arrival turn: `3`
- turn-by-turn match: `no`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=3 eta=3` | `10,10 order=move spd=3 eta=3` |
| 1 | `12,10 order=move spd=3 eta=2` | `12,10 order=move spd=3 eta=2` |
| 2 | `15,10 order=move spd=3 eta=1` | `15,10 order=move spd=3 eta=1` |
| 3 | `16,10 order=move spd=3 eta=arrived` | `16,10 order=hold spd=0 eta=arrived` |

## speed3-diagonal

- speed: `3`
- start: `10,10`
- target: `16,16`
- initial Rust ETA: `3`
- Rust arrival turn: `3`
- Classic arrival turn: `3`
- turn-by-turn match: `no`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=3 eta=3` | `10,10 order=move spd=3 eta=3` |
| 1 | `12,12 order=move spd=3 eta=2` | `11,11 order=move spd=3 eta=2` |
| 2 | `15,15 order=move spd=3 eta=1` | `14,14 order=move spd=3 eta=1` |
| 3 | `16,16 order=move spd=3 eta=arrived` | `16,16 order=move spd=3 eta=arrived` |

## speed6-diagonal

- speed: `6`
- start: `10,10`
- target: `16,16`
- initial Rust ETA: `2`
- Rust arrival turn: `2`
- Classic arrival turn: `2`
- turn-by-turn match: `no`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=6 eta=2` | `10,10 order=move spd=6 eta=2` |
| 1 | `15,15 order=move spd=6 eta=1` | `14,14 order=move spd=6 eta=1` |
| 2 | `16,16 order=move spd=6 eta=arrived` | `16,16 order=hold spd=0 eta=arrived` |

## speed1-diagonal

- speed: `1`
- start: `10,10`
- target: `13,13`
- initial Rust ETA: `4`
- Rust arrival turn: `4`
- Classic arrival turn: `5`
- turn-by-turn match: `no`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=1 eta=4` | `10,10 order=move spd=1 eta=4` |
| 1 | `10,10 order=move spd=1 eta=3` | `10,10 order=move spd=1 eta=4` |
| 2 | `11,11 order=move spd=1 eta=2` | `11,11 order=move spd=1 eta=1` |
| 3 | `12,12 order=move spd=1 eta=1` | `11,11 order=move spd=1 eta=3` |
| 4 | `13,13 order=move spd=1 eta=arrived` | `12,12 order=move spd=1 eta=2` |
| 5 | `13,13 order=move spd=1 eta=arrived` | `13,13 order=move spd=1 eta=arrived` |
| 6 | `13,13 order=move spd=1 eta=arrived` | `13,13 order=move spd=1 eta=arrived` |

## speed3-shallow

- speed: `3`
- start: `10,10`
- target: `16,12`
- initial Rust ETA: `3`
- Rust arrival turn: `3`
- Classic arrival turn: `3`
- turn-by-turn match: `no`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=3 eta=3` | `10,10 order=move spd=3 eta=3` |
| 1 | `12,12 order=move spd=3 eta=2` | `12,11 order=move spd=3 eta=1` |
| 2 | `15,12 order=move spd=3 eta=1` | `15,12 order=move spd=3 eta=1` |
| 3 | `16,12 order=move spd=3 eta=arrived` | `16,12 order=hold spd=0 eta=arrived` |

## speed3-steep

- speed: `3`
- start: `10,10`
- target: `12,16`
- initial Rust ETA: `3`
- Rust arrival turn: `3`
- Classic arrival turn: `3`
- turn-by-turn match: `no`

| turn | Rust | Classic |
| ---: | --- | --- |
| 0 | `10,10 order=move spd=3 eta=3` | `10,10 order=move spd=3 eta=3` |
| 1 | `12,12 order=move spd=3 eta=2` | `11,12 order=move spd=3 eta=1` |
| 2 | `12,15 order=move spd=3 eta=1` | `12,15 order=move spd=3 eta=1` |
| 3 | `12,16 order=move spd=3 eta=arrived` | `12,16 order=hold spd=0 eta=arrived` |
