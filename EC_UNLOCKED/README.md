# EC_UNLOCKED

Curated runnable plain-MZ copies of the Esterian Conquest v1.5 DOS executables.

This directory now lives at the project root so docs and RE workflows can
reference the unlocked binaries with short stable paths such as
`EC_UNLOCKED/ECMAINT.EXE`.

## What are these?

The shipped `ECGAME.EXE`, `ECMAINT.EXE`, and `ECUTIL.EXE` in
`original/v1.5/` are wrapped in an encrypted LZEXE 0.91 stub. Despite
the LZEXE label, the wrapper is a **PRNG-based stream cipher** (not LZ
compression) — the body is encrypted, not compressed. The stub also
includes anti-disassembly tricks (`EB FF` overlapping instructions),
three nested XOR decryption layers, an IVT-dependent anti-emulation
sled, and an "EAT SHIT AND DIE" anti-tamper hash check.

These unlocked files remove the encrypted stub from the shipped DOS binaries,
but the three executables are not rebuilt the same way:

- `ECMAINT.EXE` and `ECUTIL.EXE` come from the early post-decrypt
  `*_CLEAN.EXE` captures under `tools/unlzexe/`
- `ECGAME.EXE` comes from the larger memdump-extracted `ECGAMEU.EXE`, with
  its MZ file-size fields corrected so DOSBox-X loads the full recovered image

The preserved `tools/unlzexe/*U.EXE` artifacts keep their historical extraction
state for RE work. `EC_UNLOCKED/` is the curated runnable output set.

Supporting extraction scripts, live-memory captures, and preserved sandbox
artifacts live under [`tools/unlzexe/`](../tools/unlzexe/).

## How they differ from the originals

| Property | Original (shipped) | Unlocked |
|---|---|---|
| Format | MZ + encrypted LZEXE stub | Plain MZ |
| `file` output | `LZEXE v0.91 compressed` | `MS-DOS executable, MZ for MS-DOS` |
| Header | 512-byte oversized | Standard 32-byte |
| Body | Stream-cipher encrypted | Plaintext code + data |
| MZ relocations | 0 (TP7 handles fixups internally) | 0 (same) |
| Runs in DOSBox-X | Yes (stub decrypts at load) | Yes (curated `EC_UNLOCKED/` copies) |
| Runs in dosemu2 | No (VM86 incompatible stub) | Untested |
| Ghidra import | Requires memory dump extraction | Direct import works |

When run under the **original filename** (e.g., `ECMAINT.EXE`, not
`ECMAINT_CLEAN.EXE`), these preserve the original DOS filename-dependent
load behavior. The filename matters because DOS includes it in the
environment block, which shifts the load segment and affects Turbo Pascal
7.0's internal segment fixups.

## How they were unlocked

1. Captured the fully decrypted stub code from DOSBox-X guest RAM using
   a `memory file` timing poll (detect "EAT SHIT AND DIE" appearing at
   stub+0x77 after the XOR layers complete but before self-destruct).

2. Disassembled the stub's stream cipher at offsets +0xFF to +0x151:
   a PRNG mixing loop (ROL/ROR/XOR/RCL/RCR chain on AX+BP) that
   XOR-encrypts each body byte via XCHG+XOR with a feedback register.

3. Extracted the decrypted program bodies from DOSBox-X 640KB memory
   dumps using `tools/unlzexe/unwrap_memdump.py`, which reverses Turbo
   Pascal 7.0 segment fixups via frequency analysis.

4. Prepended original MZ headers recovered from the plaintext data area
   at stub+0x1B5 (outside all encryption ranges).

5. Rebuilt the runnable `EC_UNLOCKED/` set with:

   ```bash
   python3 tools/unlzexe/rebuild_unlocked.py --verify
   ```

   This step copies the known-good clean `ECMAINT` / `ECUTIL` images and
   repairs `ECGAMEU.EXE`'s MZ size fields so DOS loads the full extracted
   image instead of the old truncated header-defined prefix.

Full technical details in `docs/dev/dosemu2-vm86-findings.md`.
