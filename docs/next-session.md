# Next Session

Use this as the restart point instead of reconstructing the full thread.

## Current State

The active reverse-engineering target is `ECMAINT`, treated as a deterministic
black box. The current investigation focus is **Starbases** (`BASES.DAT` and
Guard Starbase order `0x04`).

**Highest-confidence planet model (Definitive):**

- `PLANETS.DAT[0x00]`: X coordinate (u8)
- `PLANETS.DAT[0x01]`: Y coordinate (u8)
- `PLANETS.DAT[0x04..0x09]`: **Factories** (48-bit Borland Pascal Real)
- `PLANETS.DAT[0x0A..0x0D]`: **Stored Goods** (32-bit LongInt)
- `PLANETS.DAT[0x0E]`: planet tax rate (synced from empire during maintenance)
- `PLANETS.DAT[0x52..0x57]`: **Population** (48-bit Borland Pascal Real)
- `PLANETS.DAT[0x58]`: **Armies** (u8)
- `PLANETS.DAT[0x5A]`: **Ground Batteries** (u8)
- `PLANETS.DAT[0x5D]`: **Owner Empire** (u8, 1-indexed; 0 = unowned)

**Highest-confidence fleet model (Definitive):**

- `FLEETS.DAT[0x0B..0x0C]`: current X, Y coordinates
- `FLEETS.DAT[0x1F]`: standing order (`4`=Guard Starbase, `5`=Sentry, `6`=Bombard, `7`=Invade, `8`=Blitz)
- `FLEETS.DAT[0x20..0x21]`: target X, Y coordinates
- `FLEETS.DAT[0x22]`: mission parameter (starbase number for order `0x04`)
- `FLEETS.DAT[0x23]`: mission parameter (must be `0x01` for Guard Starbase)
- `FLEETS.DAT[0x24]`: **Scouts** (u8)
- `FLEETS.DAT[0x25]`: **ROE** (Rules of Engagement)
- `FLEETS.DAT[0x26..0x27]`: **Battleships** (u16)
- `FLEETS.DAT[0x28..0x29]`: **Cruisers** (u16)
- `FLEETS.DAT[0x2A..0x2B]`: **Destroyers** (u16)
- `FLEETS.DAT[0x2C..0x2D]`: **Troop Transports** (u16)
- `FLEETS.DAT[0x2E..0x2F]`: **Armies** loaded on transports (u16)
- `FLEETS.DAT[0x30..0x31]`: **ETACs** (Colonization ships) (u16)

**Starbase model (New):**

- `BASES.DAT`: 35 bytes per record, mirrors truncated `FLEETS.DAT` layout
- `BASES.DAT[0x0B..0x0C]`: starbase X, Y coordinates
- `BASES.DAT[0x22]`: owner empire number
- `PLAYER.DAT[0x44..0x45]`: empire starbase count (u16)
- Guard Starbase (`0x04`) is persistent — not consumed by maintenance

**Key PLAYER.DAT fields:**

- `PLAYER.DAT[0x00]`: active/occupied flag
- `PLAYER.DAT[0x01..0x1A]`: player handle (26 bytes, padded)
- `PLAYER.DAT[0x1B..0x2E]`: empire name
- `PLAYER.DAT[0x44..0x45]`: **starbase count** (u16)
- `PLAYER.DAT[0x46..0x47]`: unknown — set to `0x01` by first maintenance pass
- `PLAYER.DAT[0x4E..0x4F]`: last run year (u16)
- `PLAYER.DAT[0x51]`: tax rate (u8)
- `PLAYER.DAT[0x52..0x55]`: treasury (u32 LongInt)

## Starbase Investigation Status

### Confirmed

- `BASES.DAT` 35-byte record format decoded (see `RE_NOTES.md`)
- `PLAYER.DAT[0x44]` = starbase count — essential for ECMAINT lookup
- `FLEETS.DAT[0x23]` = must be exactly `0x01` for Guard Starbase to resolve
- Guard Starbase is persistent (fleet order + BASES.DAT unchanged after maint)
- When lookup fails: ECMAINT zeroes BASES.DAT, clears fleet order, writes error
- Full init-based fixture with all 3 patches verified end-to-end (2 passes)
- Pre/post fixtures preserved in `fixtures/ecmaint-starbase-{pre,post}/v1.5/`
- Persistence confirmed across 2 maintenance passes (only CONQUEST.DAT year changes)

### New observation

- `PLAYER.DAT[0x46]` changes from `0x00` to `0x01` during first maintenance pass.
  Does not change on subsequent passes. Possibly a "maint has run" flag.

### NOT yet confirmed

- The meaning of `FLEETS.DAT[0x23]` is still unknown

## Next Steps

1. **Movement math**: set a fleet to Move Only (order `1`) with known speed and
   observe coordinate deltas across maintenance passes.
2. **Investigate `FLEETS.DAT[0x23]`**: create a second starbase and
   cross-reference values to determine if this is an empire ID, a base index,
   or something else.
3. **Investigate `PLAYER.DAT[0x46]`**: test whether this is a boolean flag
   or a counter by observing it across different scenarios.
4. **Rogue/AI empire behavior**: observe what ECMAINT does for non-player empires.
5. **IPBM resolution**: planetary bombardment missiles — untouched so far.

## Standard Runtime Command

See `docs/dosbox-workflow.md` for the full DOSBox-X ECMAINT testing workflow.

Quick reference:

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
  -c "exit"
```
