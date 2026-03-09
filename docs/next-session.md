# Next Session

Use this as the restart point instead of reconstructing the full thread.

## Current State

The active reverse-engineering target is `ECMAINT`, treated as a deterministic
black box:

1. start from a preserved pre-maint fixture
2. patch one controlled state variable
3. run original `ECMAINT /R` twice under `DOSBox-X`
4. diff fleet state, planet state, and result files
5. preserve only distinct outcomes

Current highest-confidence combat model:

- `PLANETS.DAT[0x04..0x09]` is a 48-bit Borland Pascal `Real` (6 bytes)
  representing `present` capacity (baseline `100.0`).
- `PLANETS.DAT[0x0A..0x0D]` is a 4-byte field, possibly a `LongInt` or part of a
  larger structure (baseline `0`).
- `PLANETS.DAT[0x0E]` is a strong defense-field candidate (ground batteries?).
  Baseline `4` for mature worlds, `0xff` caused total fleet destruction.
- `PLANETS.DAT[0x5A]` is the strongest current army/defender-count candidate.
- `PLANETS.DAT[0x58]` modulates both world damage and part of attacker losses.
- the dense world-defense block is now clearly `PLANETS.DAT[0x04..0x0E]`.

## Latest Commits

Recent combat-isolation commits:

- `2c5a19b` `Add army1 zero-dev bombardment fixtures`
- `93bba6d` `Add 0x0E bombardment defense variant`
- `b1683f7` `Add 0x08 bombardment defense variant`
- `93bba6d` `Add 0x09 bombardment defense variant` (Actually part of the 0x04-0x09 Real)

## Best Baseline

Use this preserved pre-maint scenario as the next baseline:

- `fixtures/ecmaint-bombard-army1-dev0-pre/v1.5/`

## Next Experiment

Recommended next byte:

- `PLANETS.DAT[0x0A]`

Why this next:

- We know `0x04..0x09` is a `Real`.
- `0x0A..0x0D` is the remaining 4-byte gap before the `0x0E` defense byte.
- We need to confirm if `0x0A..0x0D` is a `LongInt` (like stored goods/production) or another floating point type.

Suggested procedure:

1. copy `fixtures/ecmaint-bombard-army1-dev0-pre/v1.5/` to a throwaway `/tmp`
   directory
2. patch only target planet record `13` (zero-based), byte `0x0A` to `0x01`
3. run `ECMAINT /R` twice under the known-good `DOSBox-X` invocation
4. inspect:
   - fleet record `2` (zero-based), the attacking fleet
   - planet record `13` (zero-based), the target world
   - `MESSAGES.DAT`
   - `RESULTS.DAT`

## Standard Runtime Command

The established maintenance command is:

```bash
xvfb-run -a /tmp/dosbox-x/src/dosbox-x \
  -defaultconf \
  -nopromptfolder \
  -defaultdir /tmp/SCENARIO_DIR \
  -set "dosv=off" \
  -set "machine=vgaonly" \
  -set "core=normal" \
  -set "cputype=386_prefetch" \
  -set "cycles=fixed 3000" \
  -set "xms=false" \
  -set "ems=false" \
  -set "umb=false" \
  -set "output=surface" \
  -c "mount c /tmp/SCENARIO_DIR" \
  -c "c:" \
  -c "ECMAINT /R" \
  -c "ECMAINT /R" \
  -c "exit"
```
