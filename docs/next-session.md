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
- Important limitation:
  - `ECMAINT.EXE` is still LZEXE-packed, so current Ghidra output only sees the loader stub.
  - The first post-script pass produced only one recovered function (`entry`) and no useful strings.

**ECMAINT File-I/O Trace (New):**
- Initial runtime load order:
  - `CONQUEST.DAT`
  - `SETUP.DAT`
  - `PLAYER.DAT`
  - `PLANETS.DAT`
  - `FLEETS.DAT`
  - `BASES.DAT`
  - `IPBM.DAT`
- Trace-backed runtime record sizes:
  - `PLAYER.DAT` = `4 x 110`
  - `PLANETS.DAT` = `20 x 97`
  - `FLEETS.DAT` = `16 x 54`
  - `BASES.DAT` = `35`-byte records
- This runtime evidence supersedes the older `PLAYER.DAT = 5 x 88` guess.

**Starbase 2 Integrity Gate (New):**
- The failing multi-starbase test case aborts immediately after the initial
  read sweep of `PLAYER.DAT`, `PLANETS.DAT`, `FLEETS.DAT`, and `BASES.DAT`.
- It writes only to `ERRORS.TXT`; it does **not** reach the normal maintenance
  writeback/report pipeline.
- `ERRORS.TXT` reports:
  - `Game file(s) missing or failed integrity check!`
  - `Attempting to restore game from last saved point...`
  - `Backup game file(s) missing or failed integrity check`
  - `Maintenance aborting...`
  - `Unable to restore previous game - maintenance aborting`
- Practical conclusion:
  - the Starbase 2 blocker is a **front-loaded cross-file integrity validator**,
    not a late Guard Starbase resolution branch.

**ECMAINT Live Dump (New):**
- The productive path is now a DOSBox-X debugger memory dump, not more blind
  packer guessing.
- Working breakpoint recipe:
  - launch with `DEBUGBOX ECMAINT /R`
  - set `BPINT 21 3D`
  - when it breaks on the first file open, run `DOS MCBS`
  - dump the live block with `MEMDUMPBIN 0814:0000 97EB0`
- Confirmed dump file:
  - `/tmp/ecmaint-debug/MEMDUMP.BIN`
- Best current anchors inside the live image:
  - `0x26B86..0x26D97`: backup/primary filename tables and integrity strings
  - `0x26D98`: likely integrity/restore procedure start
  - `0x2841B..0x284E5`: `main.tok` startup guard strings including
    `Performing integrity check of game files...`
- Raw-binary Ghidra import of the dump also works:
  - project: `.ghidra/projects/ecmaint-live`
  - recovered functions: `280`
  - Ghidra anchor addresses:
    - `2000:6d98` for the integrity cluster
    - `2000:841b` for the `main.tok` startup-guard cluster
  - caveat: `2000:6d98` was not auto-promoted to a function, so it likely needs
    manual code/data carving
- First manual disassembly result:
  - linear `0x26D9B` is a top-level integrity/restore routine
  - `[bp+4] = 0` validates the primary state
  - on failure it recursively calls itself with argument `1` for the
    backup/restore-side path
  - helper `0x25EE4` is the first major validator under it and immediately
    checks structures matching `PLAYER.DAT` (`110` bytes), `PLANETS.DAT`
    (`97` bytes), and `FLEETS.DAT` (`54` bytes)
  - the next phase inside `0x25EE4` reads `BASES.DAT` (`35` bytes) using
    `PLAYER.DAT[0x44]` as the base-record selector
  - after loading that base record, it compares one base byte against the
    current player index before accepting the relation
  - targeted repro in `tools/test_starbase2_baseid_gate.py` confirms the key
    byte is `BASES[0x04]`:
    - base 2 with `0x04 = 0x02` => integrity abort
    - base 2 with `0x04 = 0x01` => accepted and normalized back to one base
    - changing duplicate-record slot byte `BASES[0x00]` does not affect this
      result

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

1. **Name and carve the integrity entry points in Ghidra**: create a function at linear `0x26D9B` / `2000:6d9b`, then label helper `0x25EE4` and the recursive backup path.
2. **Compare early validation traces**: run a known-good Guard Starbase baseline and diff its initial read/validation phase against the failing Starbase 2 scenario.
3. **Find the remaining second-base precondition**: `BASES.DAT[0x04] = 0x02` is necessary to represent a true second base, but the validator still rejects the current synthetic record set when that value advances to `2`.
4. **IPBM resolution**: investigate planetary bombardment missiles — still untouched in preserved fixtures, and `IPBM.DAT` is currently 0 bytes in all repo fixture families.
5. **Build queue mechanics (Partially Solved)**: When a build order finishes, the newly constructed ships are moved into the planet's **Stardock** (`PLANETS.DAT[0x38]` and `0x4C`). They do not immediately form a fleet in `FLEETS.DAT` until they are manually "Commissioned" by the player. We need to map out exactly how `0x38` and `0x4C` encode multiple ships/types.

## Standard Runtime Command

See `docs/dosbox-workflow.md`.
