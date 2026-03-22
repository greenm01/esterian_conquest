# ECMAINT Transit Scratch Audit

Focused follow-up on the remaining in-transit movement-state mismatch between
Rust and classic `ECMAINT`.

Status: archived implementation note. This material explains why Rust keeps an
internal exact-position seam, but it is not an active requirement to reproduce
every hidden classic movement byte.

Primary source for these controlled probes:

- `tools/ecmaint_transit_scratch_audit.py`

## What This Audit Establishes

- In the controlled transit turns checked so far, classic leaves
  `0x19..0x1e = 00/00/00/00/00/00`.
- That zeroed transit window is not limited to one mission family. It appears
  in the current controlled probes for:
  - `MoveOnly`
  - `PatrolSector`
  - `GuardBlockadeWorld`
  - `GuardStarbase`
- Rust's current `0x1a..0x1e` exact-position encoding is therefore a pragmatic
  internal movement/ETA seam, not a recovered classic byte model.
- More importantly, classic one-shot movement completion is **not** keyed from
  the first rounded target-sector hit alone. Classic can display a fleet in the
  target sector while still keeping `MoveOnly` active for later maintenance
  passes.

## Confirmed Controlled Cases

### `MoveOnly` speed 3 diagonal

- start: `10,10`
- target: `16,16`
- classic visible trace: `10,10 -> 11,11 -> 14,14 -> 16,16 -> 16,16`
- classic order/speed trace:
  - turn `0`: `move`, speed `3`
  - turn `1`: `move`, speed `3`
  - turn `2`: `move`, speed `3`
  - turn `3`: still `move`, speed `3`, even though visible coords are `16,16`
  - turn `4`: clears to `hold`, speed `0`

Practical consequence:

- Rust should not complete one-shot movement on the first rounded target-sector
  hit.
- The current Rust fix to key completion from the hidden exact path endpoint is
  enough to match this controlled `speed=3` diagonal case.

### `MoveOnly` speed 1 diagonal

- start: `10,10`
- target: `13,13`
- classic visible trace observed in controlled probing:
  - turn `0`: `10,10`
  - turn `1`: `10,10`
  - turn `2`: `11,11`
  - turn `3`: `11,11`
  - turn `4`: `12,12`
  - turn `5`: `13,13`
  - turn `6`: `13,13`
  - turn `7`: `13,13`
- classic order/speed trace:
  - turn `5`: still `move`, speed `1`
  - turn `6`: still `move`, speed `1`
  - turn `7`: clears to `hold`, speed `0`

Practical consequence:

- The hidden continuity/completion state is still **not fully recovered**.
- The current Rust exact-position seam improves the `speed=3` diagonal case,
  but it does not yet explain or mirror the slower diagonal completion delay.
- That remaining low-speed quirk is documented here, but it is not by itself a
  reason to keep reshaping the Rust engine unless it proves materially
  important to gameplay.

## Standing-Mission Transit Byte Result

The current controlled standing-mission transit probes still support the same
byte-level conclusion:

- `PatrolSector`, `GuardBlockadeWorld`, and `GuardStarbase` use the expected
  in-transit `0x0d..0x12` motion window
- classic still leaves `0x19..0x1e` zero during those transit turns
- Rust still stores its internal exact position there instead

That mismatch remains acceptable as an internal seam for now, but it should be
documented as Rust-owned state rather than described as recovered classic
semantics.

## Current Rust Consequence

The safest current interpretation is:

- keep the Rust exact-position seam because it buys correct geometry and ETA in
  many controlled cases
- do not describe `0x1a..0x1e` as classic exact-position bytes
- treat one-shot completion as driven by hidden exact-path completion rather
  than by the first rounded target-sector hit
- keep the low-speed diagonal completion path documented as background evidence,
  not as an active blocker

In short: this is a useful explanation of why Rust carries an internal movement
seam, not a mandate to keep chasing every hidden classic transit byte.
