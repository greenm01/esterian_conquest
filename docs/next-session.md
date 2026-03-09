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

- `PLANETS.DAT[0x5A]` is the strongest current army/defender-count candidate
- `PLANETS.DAT[0x58]` modulates both world damage and part of attacker losses
- `PLANETS.DAT[0x0E]` is a strong secondary defense-field candidate
- `PLANETS.DAT[0x08]` is also part of the same defense/resource cluster
- the dense world-defense block is now clearly `PLANETS.DAT[0x04..0x0E]`

## Latest Commits

Recent combat-isolation commits:

- `2c5a19b` `Add army1 zero-dev bombardment fixtures`
- `93bba6d` `Add 0x0E bombardment defense variant`
- `b1683f7` `Add 0x08 bombardment defense variant`

## Best Baseline

Use this preserved pre-maint scenario as the next baseline:

- `fixtures/ecmaint-bombard-army1-dev0-pre/v1.5/`

Why:

- it keeps `0x58 = 0`
- it keeps `0x5A = 1`
- it is the cleanest current baseline for isolating one more byte in the
  defender-world block

## Next Experiment

Recommended next byte:

- `PLANETS.DAT[0x09]`

Why this next:

- `0x08` already proved significant
- `0x09` is adjacent and still uncontrolled
- isolating it should tell us whether the `0x08..0x09` pair acts like one
  combined field or two separate contributors

Suggested procedure:

1. copy `fixtures/ecmaint-bombard-army1-dev0-pre/v1.5/` to a throwaway `/tmp`
   directory
2. patch only target planet record `13` (zero-based), byte `0x09`
3. run `ECMAINT /R` twice under the known-good `DOSBox-X` invocation
4. inspect:
   - fleet record `2` (zero-based), the attacking fleet
   - planet record `13` (zero-based), the target world
   - `MESSAGES.DAT`
   - `RESULTS.DAT`
5. preserve the fixture pair only if the outcome is distinct from:
   - `army1+dev0`
   - `army1+dev0+0x0E=0x0c`
   - `army1+dev0+0x08=0x00`

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

## What To Update After The Next Experiment

If the next experiment produces a distinct result:

- add the new fixture family under `fixtures/`
- add one `ec-data` test in `rust/ec-data/src/lib.rs`
- update:
  - `RE_NOTES.md`
  - `docs/ecmaint-combat-reference.md`
  - this file, if the recommended next byte changes
