#!/usr/bin/env python3

import sys

def main():
    try:
        with open('original/v1.5/ECMAINT.EXE', 'rb') as f:
            data = f.read()
    except FileNotFoundError:
        print("original/v1.5/ECMAINT.EXE not found")
        return

    # In Borland Pascal, strings are usually length-prefixed (e.g., \x0AStardate: ).
    # Let's search for "Stardate: " with and without prefix, and also just "Stardate".
    
    patterns = {
        "BP Stardate: ": b'\x0AStardate: ',
        "BP Stardate:": b'\x09Stardate:',
        "BP Stardate": b'\x08Stardate',
        "C Stardate: ": b'Stardate: \x00',
        "Stardate: ": b'Stardate: ',
        "Stardate:": b'Stardate:',
    }
    
    for name, pattern in patterns.items():
        idx = 0
        while True:
            idx = data.find(pattern, idx)
            if idx == -1:
                break
            
            # Found string
            print(f"\nFound '{name}' at absolute offset 0x{idx:05X}")
            
            # Determine the offset within a 64K segment.
            # Usually BP puts strings in the code segment (or a typed constant data segment).
            # The exact DS/CS base is tricky, but often the 16-bit pointer is just (idx % 0x10000)
            # or it's relative to some section. Let's just search the last 2 bytes of the address,
            # and maybe the address +/- a few typical offsets like 0, 0x10, 0x100 etc., but simple modulo is common
            
            # Try finding 16-bit pointers to idx
            target_val = idx & 0xFFFF
            target_bytes = bytes([target_val & 0xFF, (target_val >> 8) & 0xFF])
            
            print(f"  Searching for 16-bit pointer 0x{target_val:04X} ({target_bytes.hex()})")
            ptr_idx = 0
            hits = []
            while True:
                ptr_idx = data.find(target_bytes, ptr_idx)
                if ptr_idx == -1:
                    break
                hits.append(ptr_idx)
                ptr_idx += 1
                
            if hits:
                for hit in hits:
                    print(f"    Possible pointer at absolute offset 0x{hit:05X}")
            else:
                print("    No exact 16-bit pointers found.")
                
            # If the string is in a segment loaded at e.g. 0x10000 (which is 1000:0000 in memdump), 
            # its relative offset is just idx % 0x10000
            rel_offset = idx % 0x10000
            if rel_offset != target_val:
                rel_bytes = bytes([rel_offset & 0xFF, (rel_offset >> 8) & 0xFF])
                print(f"  Searching for segment-relative 16-bit pointer 0x{rel_offset:04X} ({rel_bytes.hex()})")
                ptr_idx = 0
                rel_hits = []
                while True:
                    ptr_idx = data.find(rel_bytes, ptr_idx)
                    if ptr_idx == -1:
                        break
                    rel_hits.append(ptr_idx)
                    ptr_idx += 1
                for hit in rel_hits:
                    print(f"    Possible pointer at absolute offset 0x{hit:05X}")

            idx += 1

if __name__ == '__main__':
    main()
