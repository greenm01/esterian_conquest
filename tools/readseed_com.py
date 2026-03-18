#!/usr/bin/env python3
"""Generate READSEED.COM — a tiny DOS .COM stub that reads Borland Pascal
RandSeed from a known data-segment address and writes it to SEED.BIN.

The .COM runs in the same DOS session as ECMAINT, so the ECMAINT data
segment is still intact in conventional memory.  The program reads 4 bytes
from the absolute real-mode address 3529:03A6 (ECMAINT DS:RandSeed) and
writes them to a new file SEED.BIN in the current directory.

Usage:
    python3 tools/readseed_com.py [output_path]

Default output: tools/READSEED.COM
"""

from __future__ import annotations

import struct
import sys
from pathlib import Path


# --- x86 real-mode machine code for a .COM program ---
#
# .COM files load at CS:0100h.  We need to:
#   1. Point DS to 3529h (ECMAINT's data segment at runtime)
#   2. Read 4 bytes at DS:03A6h into a buffer
#   3. Create file SEED.BIN (INT 21h / AH=3Ch)
#   4. Write 4 bytes (INT 21h / AH=40h)
#   5. Close file (INT 21h / AH=3Eh)
#   6. Exit (INT 20h)
#
# We store the filename and 4-byte buffer after the code.

def build_readseed_com(ds_segment: int = 0x3529, randseed_offset: int = 0x03A6) -> bytes:
    """Assemble the READSEED.COM binary."""
    code = bytearray()

    # Save original DS (COM segment) into ES so we can access our data later
    #   PUSH DS          ; save COM segment
    #   POP ES           ; ES = COM segment (where our filename/buffer live)
    code += b'\x1e'       # PUSH DS
    code += b'\x07'       # POP ES

    # Point DS to the ECMAINT data segment
    #   MOV AX, <ds_segment>
    #   MOV DS, AX
    code += b'\xb8' + struct.pack('<H', ds_segment)  # MOV AX, imm16
    code += b'\x8e\xd8'                               # MOV DS, AX

    # Read the 4-byte RandSeed value into registers
    #   MOV AX, [03A6h]   ; low word
    #   MOV DX, [03A8h]   ; high word
    code += b'\xa1' + struct.pack('<H', randseed_offset)       # MOV AX, [03A6h]
    code += b'\x8b\x16' + struct.pack('<H', randseed_offset + 2)  # MOV DX, [03A8h]

    # Restore DS to COM segment so we can write to our local buffer
    #   PUSH ES
    #   POP DS
    code += b'\x06'       # PUSH ES
    code += b'\x1f'       # POP DS

    # Store AX,DX into our 4-byte buffer (will be patched with offset below)
    # We'll use direct offsets; buffer_offset is computed after we know code size.
    # For now, emit placeholders — we'll patch after.
    store_ax_pos = len(code)
    code += b'\xa3\x00\x00'               # MOV [buffer], AX  (placeholder)
    store_dx_pos = len(code)
    code += b'\x89\x16\x00\x00'           # MOV [buffer+2], DX (placeholder)

    # Create file: AH=3Ch, CX=0 (normal attr), DS:DX -> filename
    # DX = offset of filename in our COM segment
    create_dx_pos = len(code)
    code += b'\xba\x00\x00'               # MOV DX, <filename_offset> (placeholder)
    code += b'\xb4\x3c'                    # MOV AH, 3Ch
    code += b'\x31\xc9'                    # XOR CX, CX
    code += b'\xcd\x21'                    # INT 21h
    # AX now has the file handle (or error)
    code += b'\x72'                        # JC <exit>  (short jump, placeholder)
    jc_exit_pos = len(code)
    code += b'\x00'                        # displacement (placeholder)

    # Write 4 bytes: AH=40h, BX=handle, CX=4, DS:DX -> buffer
    code += b'\x89\xc3'                    # MOV BX, AX (file handle)
    write_dx_pos = len(code)
    code += b'\xba\x00\x00'               # MOV DX, <buffer_offset> (placeholder)
    code += b'\xb9\x04\x00'               # MOV CX, 4
    code += b'\xb4\x40'                    # MOV AH, 40h
    code += b'\xcd\x21'                    # INT 21h

    # Close file: AH=3Eh, BX=handle (still in BX)
    code += b'\xb4\x3e'                    # MOV AH, 3Eh
    code += b'\xcd\x21'                    # INT 21h

    # Exit
    exit_offset = len(code)
    code += b'\xcd\x20'                    # INT 20h

    # --- Data section ---
    # Filename: "SEED.BIN\0"
    filename_offset = len(code)
    code += b'SEED.BIN\x00'

    # 4-byte buffer for the seed value
    buffer_offset = len(code)
    code += b'\x00\x00\x00\x00'

    # --- Patch all placeholder offsets ---
    # .COM loads at 0x100, so absolute offsets are 0x100 + position_in_code
    base = 0x0100

    # MOV [buffer], AX
    struct.pack_into('<H', code, store_ax_pos + 1, base + buffer_offset)
    # MOV [buffer+2], DX
    struct.pack_into('<H', code, store_dx_pos + 2, base + buffer_offset + 2)
    # MOV DX, <filename_offset>  (for create)
    struct.pack_into('<H', code, create_dx_pos + 1, base + filename_offset)
    # MOV DX, <buffer_offset>    (for write)
    struct.pack_into('<H', code, write_dx_pos + 1, base + buffer_offset)
    # JC <exit> displacement
    code[jc_exit_pos] = exit_offset - (jc_exit_pos + 1)

    return bytes(code)


def main() -> int:
    output = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent / "READSEED.COM"
    binary = build_readseed_com()
    output.write_bytes(binary)
    print(f"Wrote {len(binary)} bytes to {output}")
    print(f"  Reads RandSeed from 3529:03A6 (4 bytes)")
    print(f"  Writes to SEED.BIN in current directory")

    # Quick disassembly summary for verification
    print(f"\nHex dump:")
    for i in range(0, len(binary), 16):
        chunk = binary[i:i+16]
        hexpart = ' '.join(f'{b:02x}' for b in chunk)
        ascpart = ''.join(chr(b) if 32 <= b < 127 else '.' for b in chunk)
        print(f"  {0x100+i:04x}: {hexpart:<48s} {ascpart}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
