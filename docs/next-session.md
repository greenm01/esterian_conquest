# Next Session

Use this as the restart point instead of reconstructing the full thread.

## Current State

The active reverse-engineering target is `ECMAINT`. 

**Movement math (Recovered):**
- Distance moved per pass = `speed / 1.5` (approximate, with turn-based rounding).
- Observed pattern for Speed 3: Turn 1 (+2), Turn 2 (+3), Turn 3 (+3).
- Observed pattern for Speed 1: Turn 1 (+1), Turn 2 (+0), Turn 3 (+1).

**Starbase Guard Order (Definitive):**
- `FLEETS.DAT[0x22]` = empire-relative starbase index.
- `FLEETS.DAT[0x23]` = must be `0x01` for resolution.
- **Auto-merge**: multiple fleets guarding the same base merge automatically.

**Rogue Empires (Confirmed):**
- `PLAYER.DAT[0x00] = 0xFF`.
- **Auto-merge**: all rogue fleets consolidate at the homeworld into one fleet.
- Order forced to `0x05` (Guard/Blockade), ROE forced to `10`.

**Planet Owner Field (Confirmed):**
- `PLANETS.DAT[0x5D]`: owner empire number (1-indexed).

## Next Steps

1. **IPBM resolution**: investigate planetary bombardment missiles — untouched so far.
2. **Investigate `PLAYER.DAT[0x46]`**: although it didn't change in the rogue test, it has been seen to change from `0x00` to `0x01`. Re-examine in different context (e.g. joined player).
3. **Build queue mechanics**: deeper investigation of queued production materialization.

## Standard Runtime Command

See `docs/dosbox-workflow.md`.
