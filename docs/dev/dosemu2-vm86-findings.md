# ECGAME.EXE under dosemu2: VM86 Compatibility Findings

Date: 2026-03-20

## Summary

ECGAME.EXE uses LZEXE 0.91 compression with an encrypted self-modifying
stub that relies on several 8086 real-mode behaviors not supported by
VM86 mode on modern x86 CPUs. dosemu2 crashes when attempting to run the
game. Three generic VM86 fixes were developed and committed to
dosemu2 (`cf2755fcc`), but the game still does not run due to additional
incompatibilities in the stub's decryption loop.

Later testing with the rebuilt unlocked `ECGAME.EXE` also showed a separate
dosemu/FreeDOS-side failure mode before game startup:
`unable to execute C:\EC\ECGAME.EXE` / `Allocation of DOS memory failed`.
So dosemu2 is still blocked even after removing the original packed stub from
the test binary.

DOSBox-X remains the only verified working runner for original EC v1.5.

Given that DOSBox-X already covers the oracle path and the native Rust client
is expected to replace the original binaries for normal use, further deep
dosemu2 compatibility work on these binaries is currently poor ROI unless a
low-cost breakthrough appears.

## LZEXE Stub Architecture

The ECGAME.EXE stub is 544 bytes and executes ~4735 instructions to
decompress the game. Key characteristics:

- **Entry point**: CS=SS, IP=0x0000, SP=0x02D5 (from MZ header)
- **XOR decryption loop** at offsets 0x0027–0x002E, key in AL=0xAD
- **`eb ff` anti-disassembly trick**: `JMP $-1` at 0x0027 overlaps with
  `DEC BX` (`FF CB`) at 0x0028, forming the decrypt loop body
- **Exit via indirect jump**: `MOV BP, SP` then `JMP [BP+0]` at 0x0033
  reads the jump target from `DS:SP` (value `0xADFE` after decryption)
- SP never drops below 0x02CB during normal execution (verified via
  `tools/unlzexe/emu8086.py`)
- IP never exceeds 0xADFE during the stub (no 16-bit IP wraparound)

## VM86 Issues Discovered

### 1. EIP Wraparound (#GP) — FIXED

**Symptom**: `general protection at 0x...: 50` (push ax) with
`EIP: xxxx:00010000`.

**Cause**: FreeDOS/fdpp sets EIP to 0x10000 instead of 0x0000 when
loading the stub entry point. On 8086, IP is 16 bits and wraps; in VM86
mode, the 32-bit EIP exceeds the 64K segment limit causing #GP.

**Fix** (do_vm86.c `vm86_GP_fault`, kvm.c main loop): Mask EIP to 16 bits
when EIP > 0xFFFF on #GP or #SS, then retry.

### 2. ESP Wraparound (#SS) — FIXED

**Symptom**: `unexpected CPU exception 0x0c` with ESP near 0.

**Cause**: PUSH with SP < 2 would decrement SP below 0; on 8086 this
wraps to 0xFFFE/0xFFFF but in VM86 it triggers #SS.

**Fix** (do_vm86.c `vm86_fault`): Emulate PUSH reg16 (0x50–0x57),
PUSHF (0x9C), and PUSH segreg (0x06/0x0E/0x16/0x1E) with 16-bit SP
wrapping when SP < 2.

**Note**: The Python emulator (`emu8086.py`) shows SP never goes below
0x02CB during the ECGAME stub, so this fix does not actually trigger for
ECGAME. It was triggered during an earlier run where FreeDOS had consumed
stack space before entering the stub.

### 3. Undocumented `8F /reg` POP (#UD) — FIXED

**Symptom**: `SIGILL while in vm86(): xxxx:02cd opcodes: 8f ad ...`

**Cause**: Opcode `8F` with ModR/M reg field != 0 (e.g., `8F /5`) is
undefined on 386+ but works as POP r/m16 on 8086/286. VM86 raises #UD.

**Fix** (do_vm86.c `vm86_fault` case 0x06): Full ModR/M decode and
emulation of `8F /1-7` as POP r/m16 with correct 16-bit effective address
calculation and segment defaults.

**Note**: The `8F` at offset 0x02CD is stack data, not executed code.
The SIGILL was reached because of a prior issue causing execution to jump
to the wrong address.

### 4. Remaining Issue: Corrupted Decryption Output — UNFIXED

**Symptom**: After the above fixes, ECGAME shows repeating
"General Protection Fault at 0586 F910" on screen — the DOS INT 0x0D
handler looping on an unrecoverable fault.

**Root cause (theory)**: The `eb ff` self-modifying decryption loop
interacts with VM86 in a way that produces different byte values than on
a real 8086. The `JMP [BP+0]` exit instruction reads a corrupted jump
target (observed `0xAC5D` vs expected `0xADFE`), sending execution into
data. This is likely due to:

- Instruction fetch/execute differences in VM86 for overlapping
  instructions (`eb ff` / `ff cb` trick)
- Possible timing or atomicity difference in the XOR-modify-execute loop
- The `FF CB` at 0x0028 being interpreted differently when entered
  mid-instruction via the `EB FF` jump

**Approaches not yet tried**:
- Falling back to dosemu2's CPU emulator (`cpu_vm emulated`) for the
  stub segment. JIT mode crashes with SIGILL in dosemu2's own code;
  interpreter mode is too slow and hangs during FreeDOS boot.
- Patching dosemu2 to detect the `eb ff` pattern and emulate it
- Using the Python `emu8086.py` to pre-decompress the stub and
  patching the EXE before loading (blocked by custom encryption)

## dosemu2 Patches

Commit `cf2755fcc` on `devel` branch adds 169 lines across two files:

- `src/base/emu-i386/do_vm86.c`: EIP wraparound in `vm86_GP_fault()`,
  SP wraparound and `8F` emulation in `vm86_fault()`
- `src/base/emu-i386/kvm.c`: EIP wraparound in KVM main exception loop

These fixes are general-purpose improvements to dosemu2's 8086
compatibility and may help other programs that rely on real-mode
wraparound behavior (common in LZEXE, PKLITE, and other DOS packers).

## Useful Tools

- `tools/unlzexe/emu8086.py` — Python 8086 emulator that correctly
  decompresses the ECGAME stub (handles `eb ff` trick). Run with:
  ```
  python3 emu8086.py /path/to/ECGAME.EXE [output.bin]
  ```
  Outputs the full 1MB memory state after decompression. Finds known
  strings in the decompressed game code.

- `tools/unlzexe/unlzexe2.c` and `unlzexe_ecm.c` — C LZEXE
  decompressors. These hang on ECGAME because the compression is
  encrypted; the standard LZEXE format is not directly readable.

## Python Emulator Bug Fix (2026-03-20)

A critical bug was found and fixed in `emu8086.py`: the JMP SHORT (`EB`)
handler had `self.ip = (self.ip + self.fetchs8()) & 0xFFFF`. Due to
Python's left-to-right evaluation, `self.ip` was read BEFORE `fetchs8()`
advanced it, producing a displacement off by one. This caused the `eb ff`
trick to loop forever instead of overlapping with the next instruction.

After fixing this, the stub runs to 1.71M instructions through multiple
self-modifying layers and CS changes (indicating the stub completed).
However, the decompressed memory contains no recognizable game strings.

**Root cause discovered**: The stub is an anti-emulation construct that
writes to segment 0 (the IVT/BIOS data area) using `ADD [BX+SI], AL`
sleds with DS=0. On a real DOS system, the IVT contains non-zero
interrupt handler addresses, so the ADD operations produce specific
results based on the system's IVT contents. In an empty emulator, the
IVT is all zeros, producing completely different output.

This means the stub's decompression is **dependent on the DOS runtime
environment** — it uses the IVT as part of its anti-tampering mechanism.
To depack ECGAME outside of DOS, we would need to populate the IVT with
realistic DOS interrupt vectors matching whatever DOS version the game
targets (likely DOS 5.0 or 6.22).

## DOSBox-X Memory Capture (2026-03-20)

Successfully captured the decompressed ECGAME conventional memory (640KB)
from a running DOSBox-X instance using `/proc/PID/mem` (requires root).

**Method**: `tools/capture_dosbox_memory.sh` and
`tools/capture_dosbox_memory_full.sh` scan DOSBox-X process memory for
regions containing the IVT (F000 segment vectors) and game strings.
The emulated RAM is at offset `0x10` within the largest region that
has `memsz=640` at BDA offset `0x413`.

**Files**:
- `tools/unlzexe/ecgame_640k.bin` — 640KB conventional memory with
  decompressed ECGAME loaded
- `tools/unlzexe/ecgame_ivt_live.bin` — IVT + BIOS data area (1280 bytes)

**Memory map** (approximate segment addresses):
- `0700` — COMMAND.COM
- `07AA` — ECGAME.EXE PSP area
- `0800`-`42B8` — ECGAME code + data (decompressed)
- `1124` — "Esterian Conquest" string (game data segment)
- `42B4` — Turbo Pascal 7.0 runtime ("Runtime error", "Borland")

This dump can be used to:
1. Feed the IVT into `emu8086.py` for correct anti-emulation behavior
2. Directly analyze the decompressed game code in Ghidra
3. Compare with `emu8086.py` output to find emulator flag bugs

## Stub Encryption Architecture (2026-03-20)

### MZ Header Contains Plaintext Decrypted Code

The ECGAME.EXE MZ header is **512 bytes** (32 paragraphs), vs the
standard 32 bytes. File offsets `0xC8`–`0x1DF` (280 bytes) contain
**plaintext 8086 code** — the fully decrypted version of what the
stub's three XOR layers would produce if they ran correctly on a real
8086.

This means the EXE ships with both the encrypted stub AND the answer
key in the header.

### Three XOR Encryption Layers

The stub code is protected by three nested XOR decryption layers:

1. **Layer 1**: `XOR` with key `0xAD`, bytes +0x0F through +0x151
2. **Layer 2**: `XOR` with key `0x3F`, bytes +0x53 through +0x150
3. **Layer 3**: Rolling `XOR` with initial key `0x25` (ROR 1 each
   iteration), bytes +0x150 down to +0x6C

Layer 1 uses the `eb ff` anti-disassembly trick: `JMP $-1` at +0x27
overlaps with `DEC BX` (`FF CB`) at +0x28, forming the decrypt loop.

### "EAT SHIT AND DIE"

After all three layers decrypt, the string **`EAT SHIT AND DIE`**
appears at stub offset +0x77, loaded via a position-independent code
trick:

```
+6C: B2 00         MOV DL, 0x00
+6E: E8 00 00      CALL +0x0071       ; push return addr, jump to +71
+71: 5B            POP BX             ; BX = 0x0071
+72: 83 C3 06      ADD BX, 6          ; BX = 0x0077 (→ the string)
+75: EB 10         JMP +0x0087        ; skip over the string

+77: "EAT SHIT AND DIE"               ; 45 41 54 20 53 48 49 54
                                       ; 20 41 4E 44 20 44 49 45

+87: F8            CLC
+88: 72 15         JC +0x009F         ; never taken (CF=0)
```

### Anti-Tamper Hash Check

The hash verification at +0x8A–+0x9D operates on this string:

```
+8A: 33 FF         XOR DI, DI         ; DI = 0
+8C: 8B F7         MOV SI, DI         ; SI = 0
+8E: B9 08 00      MOV CX, 8          ; 8 words
+91: 33 31         XOR SI, [BX+DI]    ; XOR with word from "EAT SH..."
+93: D1 CE         ROR SI, 1          ; rotate right
+95: 47            INC DI
+96: 47            INC DI             ; advance by word
+97: E2 F8         LOOP -8
+99: 81 FE B5 95   CMP SI, 0x95B5     ; ← THE DERIVED KEY
+9D: 74 27         JZ +0xC6           ; hash OK → skip to decompressor
```

**Derived hash key: `0x95B5`** — the XOR+ROR-1 accumulation over the
8 little-endian words of `"EAT SHIT AND DIE"`:

| Word | Hex    | Text | SI after XOR+ROR |
|------|--------|------|------------------|
| 0    | 0x4145 | "EA" | 0xA0A2           |
| 1    | 0x2054 | "T " | 0x407B           |
| 2    | 0x4853 | "SH" | 0x0414           |
| 3    | 0x5449 | "IT" | 0xA82E           |
| 4    | 0x4120 | " A" | 0x7487           |
| 5    | 0x444E | "ND" | 0x9864           |
| 6    | 0x4420 | " D" | 0x6E22           |
| 7    | 0x4549 | "IE" | **0x95B5**       |

If the hash fails (tampered decryption), execution falls through to
the error handler at +0x9F.

### Error Handler (+0x9F onward)

If the hash fails, the error handler:
1. Disables the floppy motor via I/O port 0x03F2
2. Checks ROM BIOS model ID byte at `FFFF:000E` for PC-AT class
3. Hijacks the NMI vector (INT 2) to point into the stub itself
4. Enters an infinite loop (`EB FE`)

This is an anti-debugging/anti-tampering response — if the
decryption produced wrong code, the machine silently hangs.

**Note**: The MZ header at file offset 0xC8–0x1DF contains a second
copy of this code (the plaintext used by `--patch-stub`). The header
copy uses different absolute offsets because it's a position-shifted
duplicate, but the logic is identical.

### Parameter Decode Loop (+0xC6 onward)

After the hash check succeeds, the code pops saved state and derives
runtime parameters from the "EAT SHIT AND DIE" string words:

```
+C6: POP [data]         ; original DS
+CA: POP [data]         ; original FLAGS
+CE: POP [data]         ; original AX
+D2: MOV SI, 0x000E     ; read pointer (8 words from the string)
+D5: MOV DI, 0x0003     ; write index
+D8: MOV AX, [BX+SI]    ; read word from "EAT SHIT..."
+DA: ROR AX, 1
+DC: XOR AL, AH
    ... (ROR/XOR derivation loop)
+F1: JNS -0x1B          ; loop while SI >= 0
+F3: OR BYTE [data], 1  ; set bit 0 of first derived byte
```

This derives 8 bytes of LZEXE decompressor parameters from the
"EAT SHIT AND DIE" string itself — the message IS the key material.

### Decompressor + Relocation Fixer

After parameter derivation, the code uses the derived values as
segment offsets and counts for the LZEXE decompressor:

- CX = 0xC360 (49,888) — decompression word count
- Relocation fixup loop (LODSW/ADD/MOV ES/ADD ES:[BX])
- REP STOSW to zero the stub area (self-destruction)
- RETF to the decompressed game entry point

The decompressor parameters come from three sources:
1. Derived from "EAT SHIT AND DIE" (via the decode loop)
2. Saved registers from the stack (pushed before encryption layers)
3. Data area at high stub offsets (decrypted by XOR layers)

Source 3 lives **beyond** the 280-byte plaintext in the MZ header and
must come from running the actual XOR layers or extracting from a
DOSBox-X memory capture.

### All Three EXEs Use Identical Encryption

All three EC executables (ECGAME, ECMAINT, ECUTIL) share:
- Identical 512-byte oversized MZ header
- Identical stub code (first 0x100 bytes)
- Same XOR layer keys and ranges
- Same "EAT SHIT AND DIE" / 0x95B5 mechanism
- Only the per-EXE data table at stub +0x1B3+ differs

This is a standardized encryption tool applied to each EXE.

### Original MZ Headers Are Plaintext in the Stub

The stub data area at +0x1B3+ is **outside all XOR encryption
ranges** (+0x1F–0x150) and therefore stored in plaintext. At offset
+0x1B5, a complete copy of the original (pre-LZEXE) MZ header is
stored, followed by the "LZ91" signature at +0x1D1.

Original program parameters extracted from stub +0x1B5:

| Field       | ECGAME  | ECMAINT | ECUTIL  |
|-------------|---------|---------|---------|
| CS:IP       | 19F8:000E | 1196:000E | 04DC:000E |
| SS:SP       | 420B:0080 | 2ED1:0080 | 0A42:0080 |
| MZ relocs   | **0**   | **0**   | **0**   |
| Code size   | 115,509 | 78,145  | 21,441  |
| e_minalloc  | 0x3FB7  | 0x318A  | 0x1BAF  |
| e_maxalloc  | 0xDFB7  | 0xD18A  | 0xBBAF  |

**Zero MZ relocations** — Turbo Pascal handles all segment fixups
internally via its runtime initialization, not through the DOS EXE
relocation mechanism.

### LZEXE Compressed Data Is Unencrypted

The compressed program data in the file (from `hdr_size` to the stub)
is stored in plaintext. Only the decompressor stub CODE is encrypted
by the XOR layers. This means a standalone LZEXE 0.91 decompressor
can decompress the data if given the correct parameters — it does not
need to run the encrypted stub at all.

### Unwrapped EXE Attempts

**Memory capture extraction** (from DOSBox-X 640K dumps): **SUCCESSFUL**.
Initial attempts failed because the captures contained post-initialization code where the Turbo Pascal 7.0 runtime had already applied internal segment fixups for the DOSBox-X load segment. When loaded at a different segment (or loaded by Ghidra at `0000`), the TP runtime double-applied fixups or the static pointers were wrong, corrupting all segment references.

We solved this by developing `tools/unlzexe/unwrap_memdump.py`. This tool:
1. Takes a live 640KB or DOSBox-X guest RAM dump.
2. Identifies the actual `load_seg` of the running program (e.g., `0824` for ECGAME).
3. Performs a frequency analysis on all 16-bit words in the memory image to identify applied segment fixups (e.g., the `System` unit segment appears thousands of times).
4. "Reverses" these fixups by subtracting the `load_seg` from all identified pointers.
5. Prepends the original MZ header values extracted from the plaintext stub at `+0x1B5`.

This produced clean, relocatable, and Ghidra-ready executables:
- `ECGAMEU.EXE` (~622KB, ~17k fixups reversed)
- `ECMAINTU.EXE` (~622KB, ~19k fixups reversed)
- `ECUTILU.EXE` (~21KB, ~437 fixups reversed)

**These EXEs are suitable for Ghidra static analysis only — they cannot
run under DOSBox-X, dosemu2, or any DOS environment.** The memory dump
captures the program image *after* the Turbo Pascal 7.0 runtime has
already initialized: DOS memory control blocks are populated, interrupt
vectors and file handles contain stale pointers from the DOSBox-X
session, and the `.data`/`.bss` segments carry baked-in runtime state.
When a DOS loader tries to execute these EXEs, the TP7 runtime
reinitializes on top of this stale state, producing hangs or crashes.

Tested 2026-03-20: `ECMAINTU.EXE /R` in DOSBox-X against a valid
`ecmaint-econ-pre` fixture hangs indefinitely — no output, no
`ERRORS.TXT`, no file modifications.

**Note on Image Size**: The MZ header inside the stub lists `ECGAME.EXE`'s original size as ~115KB (`o_cp=226`). However, the decompressed live image is much larger (~240KB+ of actual data), with `Runtime error` appearing well beyond the 115KB mark. This implies the game relies heavily on runtime memory expansion (e.g., the BSS segment or overlays) that the `o_cp` value does not fully cover, making memory dump extraction the most reliable way to get the full running image.

### On-Disk Compression Layout

Despite the `file` command identifying these as "LZEXE v0.91 compressed",
the on-disk layout is overwhelmingly flat program body with a tiny
decompressor stub appended:

| EXE     | Body (MZ hdr to stub) | Stub  | Body % |
|---------|----------------------|-------|--------|
| ECGAME  | 115,552 bytes        | 544 B | 99%    |
| ECMAINT |  78,176 bytes        | 544 B | 99%    |
| ECUTIL  |  21,472 bytes        | 544 B | 97%    |

The 544-byte stub is the encrypted LZEXE decompressor (see "LZEXE Stub
Architecture" above). The body between the MZ header and the stub is the
compressed bitstream — it is **not** stored as raw uncompressed code.
Byte comparison against memdump-extracted images shows near-total
divergence (77,790 of 78,176 bytes differ for ECMAINT).

### Bitstream Format Is NOT Standard LZEXE 0.91

**Confirmed 2026-03-20**: The standard LZEXE 0.91 decompression
algorithm (as implemented in `unlzexe_ecm.c`, `unlzexe2.c`, and all
known public `unlzexe` tools) **cannot decompress this data at any
offset**. A brute-force scan of ECGAME across 1,543 starting positions
produced zero outputs containing known program strings ("Insufficient",
"Esterian", "PLANETS.DAT", "Runtime error", "Borland").

The bitstream start offset IS deterministic: `[0x1b3]` = `cs_rel` for
all three EXEs, yielding start = `hdr_size` (0x200). The plaintext data
area at stub+0x1B3 confirms this. But the actual decompressor algorithm
is custom and lives in the **encrypted** portion of the stub (offsets
+0x00 to +0x69, ~105 bytes).

### Plaintext Stub Is Post-Decompression Code, Not the Decompressor

The 280-byte plaintext in the MZ header (file offsets 0xC8–0x1DF)
corresponds to decrypted stub offsets +0x6A through +0x178. Disassembly
confirms this region contains:

1. Hash check ("EAT SHIT AND DIE" / 0x95B5)
2. Parameter derivation (produces `lenlz`=0xB833, `decalage`=0xCEC7)
3. Dead register setup (CX=0xC360, SI=1, DI=0 — all overwritten)
4. Relocation fixup loop (skipped since `[0x1bb]`=0)
5. Self-destruct (REP STOSB zeroes 0x151 bytes at CS:0000)
6. Jump to decompressed entry point

The derived values 0xB833 and 0xCEC7 are **dead code** — they are
loaded into AX and BP at +0xF4/+0xF7 but immediately overwritten at
+0xFB (POP BP) and later (MOV AX, [0x1af]). They are never consumed.
Similarly, CX=0xC360 is overwritten by `MOV CX, [0x1bb]` at +0x103.

The actual LZ decompressor runs in the encrypted region BEFORE this
code. The stub flow is:
1. Encrypted entry → XOR decrypt layers 1-3 (IVT-dependent)
2. **Custom LZ decompressor loop** (encrypted, offsets +0x00 to +0x69)
3. Fall through to plaintext code at +0x6A
4. Hash check, parameter derivation (dead/ceremonial)
5. Relocation fixup → self-destruct → jump to decompressed program

### Stub Data Area Parameters (Plaintext, All Three EXEs)

The data area at stub+0x1B3 onwards is outside all XOR encryption
ranges and can be read directly from disk:

| Offset | Field               | ECGAME | ECMAINT | ECUTIL |
|--------|---------------------|--------|---------|--------|
| +0x1A9 | Derived bytes (disk)| 0x00000000 | 0x00000000 | 0x00000000 |
| +0x1AF | Saved AX            | 0x1950 | 0x1950  | 0x1950 |
| +0x1B3 | cs_rel (= lenlz)    | 0x1C36 | 0x1316  | 0x053E |
| +0x1B5 | Original MZ header  | (32 bytes, valid signatures) |
| +0x1BB | Reloc count         | 0x0000 | 0x0000  | 0x0000 |
| +0x1C3 | SS offset           | 0x420B | 0x2ED1  | 0x0A42 |
| +0x1C5 | SP                  | 0x0080 | 0x0080  | 0x0080 |
| +0x1CB | IP                  | 0x19F8 | 0x1196  | 0x04DC |
| +0x1CD | Reloc table adj     | 0x001C | 0x001C  | 0x001C |
| +0x1D1 | Signature           | LZ91   | LZ91    | LZ91   |

### Static Decompressor Attempts

**Standard LZEXE 0.91 algorithm**: Cannot decompress. Zero valid outputs
across 1,543 brute-force offset attempts on ECGAME. The bitstream format
is custom.

**Python `unlzexe91_ec.py`**: Uses standard LZEXE 0.91 logic. Hits
false end-markers producing ~9K-21K of garbage.

**Modified `unlzexe_ecm.c`**: Cannot read the encrypted stub parameters.

### Path Forward for Runnable EXEs

The actual decompressor algorithm is in ~105 bytes of encrypted stub
code (offsets +0x00 to +0x69). Recovery options:

1. **DOSBox-X mid-execution capture**: Break at a stub instruction
   (e.g., the hash check at CS:0x0083) BEFORE the self-destruct zeroes
   the first 0x151 bytes. Dump the decrypted decompressor code from
   CS:0x0000 to CS:0x0069. The self-destruct at +0x120 (REP STOSB,
   0x151 bytes) wipes this region, so post-run captures are useless.

2. **Fix emu8086.py**: The emulator has the encrypted stub but produces
   wrong results with IVT data loaded. Fixing the emulator's flag/
   instruction bugs would allow purely static decryption using the
   captured DOSBox-X IVT (`tools/unlzexe/ecgame_ivt_live.bin`).

3. **Plaintext cross-validation**: The plaintext at file 0xC8-0x1DF is
   the known-good decrypted output for stub offsets +0x6A-0x178. Any
   decryption attempt can be validated against these bytes.

### Decompressed Image Size (2026-03-20)

The decompressed image is MUCH larger than the compressed body:

| EXE     | Compressed | SS offset  | ~Decompressed | Ratio |
|---------|-----------|------------|---------------|-------|
| ECMAINT | 78,176 B  | 0x2ED10 B  | ~192 KB       | ~2.5x |

Confirmed via DOSBox-X `memory file` capture: "Runtime error" at
body+0x2B701 (178 KB), "Borland" at +0x2B738. The body's entropy is
7.93/8.0 (near-random), confirming real compression, not a cipher.

### Algorithm Is NOT Standard LZEXE 0.91 (Exhaustive Testing)

Tested 2026-03-20 with all combinations of:
- LSB-first vs MSB-first bit extraction
- Standard vs inverted literal/copy-back bit sense
- Standard vs inverted copy-type decision bits
- SHR-carry vs shift-then-read getbit variants
- Forward and backward reading directions
- With and without end-marker termination

Result: zero valid decompression across all variants. The bitstream
format is custom and differs from all known public LZEXE implementations.

### emu8086.py Status (2026-03-20)

Tested with `--dos-mem ecgame_ivt_live.bin` on ECMAINT:
- With IVT: 10K instructions, Layer 1 XOR runs but then jumps to
  zero memory (ADD sled at CS:D720). Body unchanged.
- Without IVT: 5K instructions, same Layer 1 result, different
  wrong jump target.
- 2M instructions with IVT: crashes at 381K insns with "Unknown
  opcode 0x65" (GS: prefix, 386-only) after jumping to DS=F000
  (BIOS ROM area).

The emulator correctly executes the Layer 1 XOR loop but diverges
at the IVT-dependent code that follows. The IVT-captured data from
DOSBox-X is not sufficient (or the emulator has flag/instruction
bugs in the self-modifying decrypt path).

### Path Forward for Runnable EXEs

Once the ~105-byte decompressor is recovered, implement it in Python
and apply to the plaintext bitstream body at offset 0x200. Combined
with the original MZ header at stub+0x1B5 and zero relocations, this
produces clean unwrapped EXEs with pristine data/BSS segments.

### Breakthrough: DOSBox-X Memory-File Timing Capture (2026-03-20)

The `memory file` option maps guest RAM to a host file that can be
read while DOSBox-X runs. By running with `cycles=fixed 50` (slow)
and polling the memory file for the "EAT SHIT AND DIE" string at
stub+0x77, we can detect the exact moment the XOR layers complete
and capture the fully decrypted stub BEFORE the self-destruct runs.

The decrypted stub reveals that the "decompressor" at +0xFF to +0x151
is a **stream cipher**, NOT LZ compression:

```
+0xFF:  MOV CX, <body_size_lo>   ; SI:CX = body size (78,176 for ECMAINT)
+0x102: MOV SI, 0x0001
+0x105: MOV ES, BP               ; ES = load_seg
+0x107: MOV DI, 0x0000           ; ES:DI = body start
+0x10A: MOV AX, [0x1A9]          ; PRNG seed from derivation
+0x10D: MOV BP, [0x1AB]
+0x111: MOV DL, AL               ; cipher feedback register
+0x113: <PRNG mixing: ROL/ROR/XOR/RCL/RCR on BH,BL,AH,AX,BP>
+0x12F: TEST AX, 0x4000          ; PRNG output bit
+0x132: JZ skip                  ; ~99% of bytes are encrypted
+0x134: XCHG DL, [ES:DI]        ; swap DL and memory byte
+0x137: XOR [ES:DI], DL          ; XOR memory with swapped DL
skip:   INC DI                   ; next byte
+0x145: SUB CX,1 / SBB SI,0     ; 32-bit counter
+0x14B: JNZ loop
```

The body on disk is encrypted with this stream cipher seeded by the
4 bytes derived from "EAT SHIT AND DIE". The "LZEXE compression" label
is deliberately misleading — the body is the **same size** before and
after decryption (no compression, only encryption).

**Working capture + build recipe for ECMAINT:**

1. Run DOSBox-X with `cycles=fixed 50` and `memory file=PATH`
2. Poll the memory file for "EAT SHIT AND DIE" at the stub location
3. When found, capture the body (at load_seg * 16)
4. Wait slightly longer, capture again to fill remaining bytes
5. Build clean MZ EXE: 32-byte header from stub+0x1B5 + decrypted body

Result: `ECMAINT_CLEAN.EXE` runs in DOSBox-X and produces near-
identical oracle output (3/6 DAT files identical, others differ by
7-51 bytes due to ~611 bytes captured from post-TP7-init memory).

The decoy plaintext in the MZ header at 0xC8-0x1DF differs from the
real decrypted stub by 49% of bytes. Key differences include:
- CX=0xC360 (decoy) vs CX=0x3160 (real) — body size word
- The PRNG mixing code is different (different ROL/ROR operands)
- The "AND DIE" portion of the string area is replaced with entry code

### Current Status of emu8086.py

New CLI flags: `--patch-stub`, `--dos-mem PATH`, `--verify PATH`,
`--max-insns N`.

**Emulator bug**: With non-zero IVT data loaded, the XOR decryption
layers take a different (incorrect) execution path — the self-modifying
code interacts with IVT values via an ADD sled at DS=0, and our
emulator produces different results than DOSBox-X's CPU. With empty
memory, CS changes after 1.71M instructions but jumps to 0000:0000
(wrong). With DOSBox-X IVT loaded, CS never changes (stuck in loop).
Root cause not yet identified — likely a subtle instruction behavior
difference in the `eb ff` overlapping-instruction trick or in flag
handling during the self-modifying XOR loop.

## References

- dosemu2 source: https://github.com/dosemu2/dosemu2
- Intel 8086 behavior for `8F /reg`: all reg fields act as POP r/m16
- LZEXE 0.91 format: Fabrice Bellard's original compressor
- `eb ff` trick: common anti-disassembly technique in DOS copy protection
