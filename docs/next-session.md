# Next Session

Use this as the restart point instead of reconstructing the full thread.

## Current State

The active reverse-engineering target is `ECMAINT`, treated as a deterministic
black box.

**Highest-confidence planet model (Definitive):**
A successful heavy bombardment generated a complete combat report in `RESULTS.DAT`,
explicitly naming and defining the core planetary fields:

- `PLANETS.DAT[0x04..0x09]` is a 48-bit Borland Pascal `Real` representing **Factories** (present capacity/development).
- `PLANETS.DAT[0x0A..0x0D]` is a 32-bit `LongInt` representing **Stored Goods** (production points).
- `PLANETS.DAT[0x0E]` is a strong secondary defense field, potentially a forcefield/multiplier, as it isn't explicitly listed alongside batteries/armies in basic reports but directly scales attacker losses.
- `PLANETS.DAT[0x58]` is the **Armies** count.
- `PLANETS.DAT[0x5A]` is the **Ground Batteries** count.

This completely resolves the dense `0x04..0x5A` planet block puzzle for bombardments!

## Latest Commits

- `edd013e` `Identify 0x04-0x09 as Real, add 0x09 bombardment fixture and test`
- `73aefb7` `Update handoff for next bombardment scaling experiment`
- `[NEW]` Added heavy bombardment test proving report generation and exact byte mappings.

## Next Experiment

Goal: Decode `ECMAINT`'s handling of planetary invasions or fleet-vs-fleet combat.

Now that Bombardment is mapped, we can move to the next interaction phase.

Suggested path: Fleet Invasion
1. Use the `heavy` attacker baseline, but change the fleet's order to `Invade` (order code `10`).
2. Run `ECMAINT` on the target planet.
3. Observe how `ECMAINT` resolves ground combat using the known `Armies` (`0x58`) and `Ground Batteries` (`0x5A`) fields.
4. Verify if ownership changes and how `RESULTS.DAT` reports the invasion.

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
