#!/usr/bin/env python3
"""Unwrap a DOS memory dump into a relocatable MZ EXE.

Identifies segment fixups by frequency analysis and undoes them
based on a provided load segment.
"""
import argparse
import struct
from collections import Counter
from pathlib import Path

def unwrap(dump_path, load_seg, out_path, mz_header_info=None, threshold=5):
    data = Path(dump_path).read_bytes()
    
    # Image starts after PSP (0x100 bytes = 0x10 paragraphs)
    image_start = load_seg << 4
    image = bytearray(data[image_start:])
    
    print(f"Image starts at physical 0x{image_start:x}")
    print(f"Analyzing {len(image)} bytes for segment fixups (base 0x{load_seg:04x})...")
    
    # Identify candidate segments to undo
    words = struct.unpack(f'<{len(image)//2}H', image[:len(image)//2*2])
    # Assume program doesn't exceed 384KB (0x6000 paragraphs)
    candidates = Counter(w for w in words if load_seg <= w <= load_seg + 0x6000)
    
    to_undo = set()
    for w, freq in candidates.most_common():
        if freq >= threshold:
            # Heuristic: exclude common ASCII/junk
            # 0x2020 = '  ', 0x2d2d = '--', 0x20xx = ' x'
            if (w & 0xFF00) == 0x2000 or (w & 0xFF00) == 0x2d00:
                continue
            if 0x2020 <= w <= 0x7E7E: # mostly ASCII range
                # Only include if frequency is very high (likely a segment)
                if freq < threshold * 10:
                    continue
            
            to_undo.add(w)
            print(f"  Fixing segment 0x{w:04x} (rel 0x{w-load_seg:04x}), found {freq} times")

    # Undo fixups in the image
    fixed_count = 0
    for i in range(0, len(image) - 1, 2):
        w = struct.unpack_from('<H', image, i)[0]
        if w in to_undo:
            # Double check: is it likely a pointer?
            # In code: often B8 XX XX (MOV AX, imm16) or similar
            # We don't have a full disassembler here, but we can check if it's
            # after an opcode that takes a segment/immediate.
            image[i:i+2] = struct.pack('<H', w - load_seg)
            fixed_count += 1
            
    print(f"Applied {fixed_count} reverse-fixups.")
    
    # Build MZ header
    if mz_header_info:
        # mz_header_info: (cblp, cp, crlc, cparhdr, minalloc, maxalloc, ss, sp, ip, cs)
        h = mz_header_info
        header = struct.pack('<16H',
            0x5A4D, h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], 0, h[8], h[9], 0x001C, 0, 0, 0
        )
    else:
        # Default header (non-relocatable, but at least has correct entry point)
        header = struct.pack('<16H',
            0x5A4D, len(image) % 512, (len(image) + 511) // 512, 0, 2, 0xFFFF, 0xFFFF, 0, 0x100, 0, 0, 0, 0x001C, 0, 0, 0
        )
        
    Path(out_path).write_bytes(header + image)
    print(f"Written {out_path} ({len(header) + len(image)} bytes)")

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('dump', help='640k.bin dump file')
    parser.add_argument('load_seg', help='Load segment (hex, e.g. 0824)')
    parser.add_argument('output', help='Output EXE path')
    parser.add_argument('--threshold', type=int, default=5, help='Frequency threshold for fixup detection')
    args = parser.parse_args()
    
    # ECGAME original info
    ecgame_h = (341, 226, 0, 2, 16311, 57271, 16907, 128, 14, 6648)
    # ECMAINT original info
    ecmaint_h = (353, 153, 0, 2, 12682, 53642, 11985, 128, 14, 4502)
    # EC_UTIL original info
    ecutil_h = (182, 29, 0, 2, 6127, 48043, 2626, 128, 14, 1244)
    
    # Select header based on filename
    h = ecgame_h
    if 'ecmaint' in args.dump.lower() or 'ecmaint' in args.output.lower():
        h = ecmaint_h
    elif 'ecutil' in args.dump.lower() or 'ecutil' in args.output.lower():
        h = ecutil_h
    
    unwrap(args.dump, int(args.load_seg, 16), args.output, mz_header_info=h, threshold=args.threshold)
