# ECGAME.EXE under dosemu2: VM86 Compatibility Findings

Date: 2026-03-20

## Summary

ECGAME.EXE uses LZEXE 0.91 compression with an encrypted self-modifying
stub that relies on several 8086 real-mode behaviors not supported by
VM86 mode on modern x86 CPUs. dosemu2 crashes when attempting to run the
game. Three generic VM86 fixes were developed and committed to
dosemu2 (`cf2755fcc`), but the game still does not run due to additional
incompatibilities in the stub's decryption loop.

DOSBox-X remains the only working runner for ECGAME.

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

### Current Status of emu8086.py

New CLI flags added: `--patch-stub`, `--dos-mem PATH`, `--verify PATH`.

**`--patch-stub` approach**: Overlays the 280-byte plaintext from the
MZ header at CS:0000 and patches the anti-tamper JZ to JMP. The
anti-tamper check is bypassed, but the decompressor exits immediately
because the XOR-decrypted data table beyond the plaintext region is
unpopulated.

**Next steps**:
1. Extract missing data table values from the DOSBox-X capture
   (`ecgame_640k.bin`) at the stub segment's physical addresses
2. Or: run the XOR layers in the emulator first (with correct IVT),
   then let the decompressor proceed naturally
3. Or: just use the DOSBox-X 640K capture directly for analysis and
   skip offline decompression entirely

## References

- dosemu2 source: https://github.com/dosemu2/dosemu2
- Intel 8086 behavior for `8F /reg`: all reg fields act as POP r/m16
- LZEXE 0.91 format: Fabrice Bellard's original compressor
- `eb ff` trick: common anti-disassembly technique in DOS copy protection
