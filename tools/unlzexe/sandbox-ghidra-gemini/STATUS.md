# Ghidra Sandbox Status: ECMAINT Analysis

Date: 2026-03-20

## Summary
The goal of this sandbox is to perform headless Ghidra analysis on the unwrapped `ECMAINTU.EXE` binary to recover game logic and data structures.

## Achievements

### 1. Unwrapped Binary Preparation
- **Relocatable Image:** `ECMAINTU.EXE` has been fully decompressed and its Turbo Pascal 7.0 segment fixups have been reversed using `unwrap_memdump.py`.
- **Linear Memory Map:** The binary now has a clean linear memory layout starting from `0000:0000`, making Ghidra cross-references (XRefs) accurate.

### 2. Headless Ghidra Infrastructure
- **Sandbox Environment:** Dedicated project space created in `ghidra_project/`.
- **Automation:** `sandbox_analyze.sh` provides a template for running headless analysis and post-scripts.
- **Verification:** Proved that Ghidra can successfully decompile raw x86 assembly into structured C-like logic using `DecompileMaint.java`.

### 3. Initial RE Discoveries
- **Entry Point:** Mapped the standard TP7 startup sequence at file offset `0x1198E`.
- **String Table:** Located the Mission Report string block starting at file offset `0x2479A` (containing `"Scouting mission report:"`, `"Invasion mission report:"`, etc.).
- **Logic Anchors:** 
    - Identified a major candidate for the Mission Processing loop at file offset `0x21CC6` (RAM `2000:1cc6` if loaded at `2000`).
    - This function (`FUN_1000_01c0` in the current analysis) contains complex branching logic that references several global state buffers.

## Working Files
- `ECMAINTU.EXE`: The target unwrapped binary.
- `DecompileMaint.java`: Script to extract C logic from identified functions.
- `ecmaint_asm/`: Folder containing raw assembly snippets and string pointers for quick reference.

## Next Steps
1. **RTL Identification:** Apply FLIRT signatures or scripts to identify and label common Turbo Pascal Runtime Library (RTL) functions to reduce decompiler noise.
2. **Data Structure Mapping:** Use the string XRefs to identify the memory offsets for Planet and Fleet records as they are processed in the mission loop.
3. **Logic Recovery:** Focus decompilation on the Mission Processing loop (`0x21CC6`) to extract the exact mathematical rules for scout success, combat damage, and colonization.
