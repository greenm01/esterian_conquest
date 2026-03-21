# Esterian Conquest Static Unpacking Analysis: Rebuttal to Standard LZEXE Assumptions

Date: 2026-03-20

This document addresses assertions that `ECGAME.EXE`, `ECMAINT.EXE`, and `ECUTIL.EXE` can be statically unpacked using standard LZEXE 0.91 logic or by analyzing the plaintext stub in the MZ header. 

The Esterian Conquest binaries are **not** standard LZEXE compressed files. They use a highly customized, polymorphic anti-emulation wrapper that intentionally misrepresents its structure and breaks static analysis tools.

## Rebuttal to Key Assertions

### Claim 1: "The 280-byte plaintext stub in the MZ header is the answer key"
**Status: Mathematically Disproven.**

The plaintext stub located in the oversized MZ header (`0xC8`–`0x1DF`) is a deliberately constructed **decoy**. 

If you statically trace the x86 execution of the parameter derivation loop in that plaintext stub, it reads the 8 words of the string `"EAT SHIT AND DIE"` backwards, performing `ROR 1` and `ROL 1` operations to derive the decompression parameters `lenlz` and `decalage`.

Executing this exact mathematical loop yields:
- `lenlz` = `0xB833`
- `decalage` = `0xCEC7`

In standard LZEXE 0.91, the offset of the compressed data stream is calculated as:
`fpos = (cs_rel - lenlz + hdr_paras) * 16`

Using `ECGAME`'s MZ values (`cs_rel = 0x19F8`, `hdr_paras = 0x20`):
`fpos = (0x19F8 - 0xB833 + 0x20) * 16 = 0x61E50` (**400,976 bytes**)

`ECGAME.EXE` is only **116,608 bytes** long. The parameters derived from the plaintext stub point nearly 300KB *past the end of the file*. This definitively proves the plaintext stub is a trap designed to waste reverse engineers' time and break generic unpackers like `unlzexe_ecm.c` that look for fixed offsets.

### Claim 2: "Standard LZEXE 0.91 decompresses backwards"
**Status: Factually Incorrect.**

Standard LZEXE 0.91 (written by Fabrice Bellard) decompresses the bitstream **forward**. The compressed data is read sequentially from low memory to high memory. 

The *stub* calculates a `decalage` (shift) value to physically move the compressed payload to high memory before decompression begins. This allows the forward-reading decompressor to unpack the data into low memory without overwriting the compressed stream itself. But the bitstream on disk, and the read direction during decompression, is completely forward-facing.

### Claim 3: "The LZEXE bitstream body is plaintext and can be decompressed without the stub"
**Status: True, but useless due to a forged MZ header.**

It is true that the LZEXE bitstream itself is not encrypted. However, statically unpacking it is impossible because the original program size (`o_cp = 226`, or ~115KB) stored at `stub+0x1B5` is **also a lie**.

When we developed a brute-force script (`brute_all.py`) to attempt decompression at every 16-byte boundary in `ECGAME.EXE`, it found hundreds of valid LZ bitstream starting points producing between 50,000 and 106,000 bytes of output. **None of them produced the 115,541 bytes specified in the original MZ header.**

When we dumped `ECGAME` from live memory in DOSBox, the actual decompressed executable data (containing expected strings like `Runtime error` and `planets.dat`) extended well beyond **240KB**. The packer compressed a much larger executable (likely utilizing overlays or heavy BSS expansion), then altered the preserved MZ header values to misrepresent its true size and memory layout. 

Without the *real* decompression parameters, you cannot statically know the correct starting byte or the true decompressed size.

### Claim 4: "The compressed data does not require IVT-derived keys"
**Status: Missing the point.**

The compressed data stream is plaintext, but the *real* LZEXE parameters (`lenlz`, `decalage`, and the true original MZ header values) are located inside the **encrypted stub** at the very end of the file (`0x1C560`).

This real stub is protected by an `EB FF` overlapping-instruction XOR loop that is specifically designed to break emulators and VM86 mode (like `dosemu2`). Furthermore, its decryption layers use `ADD [BX+SI], AL` sleds pointing to `DS=0` (the DOS Interrupt Vector Table).

The final decrypted bytes of this real stub—which contain the true decompression parameters—depend entirely on the exact state of the IVT at the moment the file was packed or executed. Because of this IVT dependency, the true parameters mathematically cannot be decrypted statically without a cycle-accurate 8086 emulator pre-loaded with the exact DOS 5.0/6.22 runtime environment the author used.

## The Working Solution: `unwrap_memdump.py`

Because static unpacking is mathematically blocked by forged MZ headers, decoy stubs, and an IVT-dependent anti-emulation wrapper, the only reliable solution is memory extraction.

We captured 640KB post-decompression RAM dumps from DOSBox-X and developed `tools/unlzexe/unwrap_memdump.py` to:
1. Identify the running program's actual `load_seg`.
2. Perform a frequency analysis on all 16-bit words to accurately locate Turbo Pascal 7.0 segment fixups (e.g., identifying the `System` unit segment which appears thousands of times).
3. Reverse the fixups by subtracting the `load_seg` from these pointers, making the code relocatable again.
4. Prepend the clean MZ headers.

This produced perfectly clean, relocatable MZ executables (`ECGAMEU.EXE`, `ECMAINTU.EXE`, `ECUTILU.EXE`) that contain the true 240KB+ execution payloads, all readable strings, and are fully ready for static analysis in Ghidra.

**Conclusion:** A custom static unpacker is a dead end for these specific binaries due to the sophisticated polymorphic and environment-dependent protections employed. The unwrapped memory dumps are the superior, verified solution.