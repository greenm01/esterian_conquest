# Unlzexe Workbench

This directory preserves the binary-unlocking work behind the original
Esterian Conquest DOS executables.

## Canonical Assets

The top level holds the artifacts that proved most useful in day-to-day RE:

- `ECGAMEU.EXE`, `ECMAINTU.EXE`, `ECUTILU.EXE`
  unwrapped MZ executables recovered from DOSBox-X memory captures
- `ECGAME_CLEAN.EXE`, `ECMAINT_CLEAN.EXE`, `ECUTIL_CLEAN.EXE`
  clean intermediate variants used as static-analysis anchors
- `ECGAME_STATIC.EXE`
  additional static-analysis build output retained for comparison
- `ecgame_640k.bin`, `ecgame_ivt_live.bin`, `ecmaint_640k.bin`,
  `ecutil_640k.bin`
  preserved live-memory captures
- `unwrap_memdump.py`, `unlzexe91_ec.py`, `brute_lzexe.py`
  helper scripts from the unlocking work
- `unlzexe2.c`, `unlzexe_ecm.c`
  preserved unpacker baselines and experiments

The runnable unlocked binaries that the rest of the project treats as the
primary plain-MZ copies live in [`EC_UNLOCKED/`](../../EC_UNLOCKED/).

## Sandbox Layout

### `sandbox-gemini/`

Static unpacking and anti-emulation investigation.

Kept here:

- the working note `EC_UNPACK_FINDINGS.md`
- exploratory helper scripts
- a curated set of intermediate `.bin` artifacts and copied `.EXE` inputs

Ignored here:

- `brute_out_*.bin`

Those numbered files are exhaustive brute-force search byproducts. They are
kept locally because they document the search space, but they stay ignored so
they do not dominate normal repo status.

### `sandbox-ghidra-gemini/`

Headless Ghidra sandbox for the unwrapped binaries.

Kept here:

- reusable scripts
- `STATUS.md`
- preserved `.EXE`, `.asm`, and `.txt` analysis artifacts

Ignored here:

- temporary Ghidra project state
- single-run logs
- duplicate single-script scratch directories

## Policy

This tree is no longer treated as disposable scratch. Keep meaningful
unlocking artifacts, tooling, and notes. Ignore only machine-local cache,
brute-force floods, and temporary project/runtime byproducts.
