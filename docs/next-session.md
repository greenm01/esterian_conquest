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

## Next Experiment

Goal: Decode `ECMAINT`'s handling of fleet-vs-fleet combat or AI empire behavior.

Now that planet-side combat (Bombardment, Invasion) is mapped, the next phase should focus on space combat and empire management.

Suggested path: Fleet-vs-Fleet Interception
1. Set up a pre-maint scenario with two hostile fleets in the same deep-space sector (or moving through the same sector).
2. Ensure both have high combat strength (Cruisers, Destroyers, Battleships) to guarantee a decisive engagement.
3. Observe how `ECMAINT` resolves the encounter, noting losses and generation of interception reports in `MESSAGES.DAT` or `RESULTS.DAT`.
4. Validate how Rules of Engagement (`0x25`) impacts the flee/fight mechanics.

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
