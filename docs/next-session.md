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

**Highest-confidence fleet model (Definitive):**
A successful planetary invasion generated a casualty report confirming that the game engine stores ship and troop counts as **16-bit (little-endian) integers** starting at `0x24` in `FLEETS.DAT`:

- `FLEETS.DAT[0x1F]` is the standing order (e.g., `6` = Bombard, `7` = Invade, `8` = Blitz).
- `FLEETS.DAT[0x24]` is the **Scouts** count (8-bit).
- `FLEETS.DAT[0x26..0x27]` is the **Battleships** count (`u16`).
- `FLEETS.DAT[0x28..0x29]` is the **Cruisers** count (`u16`).
- `FLEETS.DAT[0x2A..0x2B]` is the **Destroyers** count (`u16`).
- `FLEETS.DAT[0x2C..0x2D]` is the **Troop Transports** count (`u16`).
- `FLEETS.DAT[0x2E..0x2F]` is the **Armies** loaded on transports (`u16`).
- `FLEETS.DAT[0x30..0x31]` is the **ETACs** (Colonization ships) count (`u16`).

## Latest Commits

- `edd013e` `Identify 0x04-0x09 as Real, add 0x09 bombardment fixture and test`
- `73aefb7` `Update handoff for next bombardment scaling experiment`
- `[NEW]` Added heavy bombardment test proving report generation and exact byte mappings.
- `[NEW]` Mapped 16-bit fleet ship capacities and Invasion orders via ECMAINT black-box testing.
- `[NEW]` Decoded Planetary Economics: Population, Factories, Stored Goods, and Treasury.

## Next Experiment

Goal: Decode `ECMAINT`'s handling of Starbases or Deep Space movement formulas.

Now that the core state (Combat, Economics, Production) is mapped, the remaining unknowns are the stationary defenses and the exact math behind movement.

Suggested path: Starbases
1. Set up a pre-maint scenario with a Starbase in a sector (`BASES.DAT`).
2. Order a fleet to `Guard Starbase` (order `4`).
3. Check how Starbases contribute to fleet defense or storage.
4. Try to reverse-engineer the `BASES.DAT` format (likely similar to planets but simpler).

Alternative path: Movement Math
1. Set a fleet to `Move Only` (order `1`) with a known speed (`0x09`) and current speed (`0x0A`).
2. Observe the coordinate delta over multiple maintenance runs.
3. Determine if movement is strictly linear or if there is a "sublight" vs "translight" transition.

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
