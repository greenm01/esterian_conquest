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

## References

- dosemu2 source: https://github.com/dosemu2/dosemu2
- Intel 8086 behavior for `8F /reg`: all reg fields act as POP r/m16
- LZEXE 0.91 format: Fabrice Bellard's original compressor
- `eb ff` trick: common anti-disassembly technique in DOS copy protection
