# Next Session

Use this as the restart point instead of reconstructing the full thread.

## Current State

The active reverse-engineering target is `ECMAINT`. 

**Headless Ghidra (Ready):**
- `tools/ghidra_ecmaint.sh` imports and analyzes `original/v1.5/ECMAINT.EXE` headlessly.
- Repo-local Ghidra state lives under `.ghidra/`; logs live under `artifacts/ghidra/ecmaint/`.
- Current confirmed baseline:
  - Loader: old-style DOS `MZ`
  - Language: `x86:LE:16:Real Mode:default`
  - MD5: `21489ef9798df77b20b7a02eb9347071`

**Movement math (Recovered):**
- Distance moved per pass = `speed / 1.5` (approximate, with turn-based rounding).
- Observed pattern for Speed 3: Turn 1 (+2), Turn 2 (+3), Turn 3 (+3).
- Observed pattern for Speed 1: Turn 1 (+1), Turn 2 (+0), Turn 3 (+1).

**Starbase Guard Order (Definitive):**
- `FLEETS.DAT[0x22]` = empire-relative starbase index.
- `FLEETS.DAT[0x23]` = must be `0x01` for resolution.
- **Auto-merge**: multiple fleets guarding the same base merge automatically.
- `PLAYER.DAT[0x46..0x47]` is **not required as a precondition** for Guard Starbase resolution and is **not specific to order `0x04`**; it normalizes to `0x0001` when ECMAINT sees a valid starbase state for the empire.
- `BASES.DAT[0x04]` behaves like the real starbase identity/number; promoting it to `0x02` is what triggers the multi-starbase integrity gate, while changing only `BASES.DAT[0x00]` is not enough.

**Rogue Empires (Confirmed):**
- `PLAYER.DAT[0x00] = 0xFF`.
- **Auto-merge**: all rogue fleets consolidate at the homeworld into one fleet.
- Order forced to `0x05` (Guard/Blockade), ROE forced to `10`.

**Planet Owner Field (Confirmed):**
- `PLANETS.DAT[0x5D]`: owner empire number (1-indexed).

## Next Steps

1. **Use headless Ghidra on Starbase 2**: locate the `ECMAINT` routines that open `BASES.DAT`, `PLAYER.DAT`, and `FLEETS.DAT`, then trace the branch that rejects `BASES.DAT[0x04] = 0x02` scenarios.
2. **Find the Starbase 2 companion structure**: `BASES.DAT[0x04] = 0x02` and `PLAYER.DAT[0x44] = 0x0002` are not sufficient by themselves, even with a second owned planet.
3. **IPBM resolution**: investigate planetary bombardment missiles — still untouched in preserved fixtures, and `IPBM.DAT` is currently 0 bytes in all repo fixture families.
4. **Build queue mechanics (Partially Solved)**: When a build order finishes, the newly constructed ships are moved into the planet's **Stardock** (`PLANETS.DAT[0x38]` and `0x4C`). They do not immediately form a fleet in `FLEETS.DAT` until they are manually "Commissioned" by the player. We need to map out exactly how `0x38` and `0x4C` encode multiple ships/types.

## Standard Runtime Command

See `docs/dosbox-workflow.md`.
