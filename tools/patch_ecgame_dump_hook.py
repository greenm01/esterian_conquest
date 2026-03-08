#!/usr/bin/env python3
from pathlib import Path
import sys


def patch_mz_size(buf: bytearray) -> None:
    size = len(buf)
    pages = (size + 511) // 512
    extra = size % 512
    buf[2] = extra & 0xFF
    buf[3] = (extra >> 8) & 0xFF
    buf[4] = pages & 0xFF
    buf[5] = (pages >> 8) & 0xFF


def main() -> int:
    if len(sys.argv) != 4:
        print("usage: patch_ecgame_dump_hook.py <input-exe> <hook-bin> <output-exe>")
        return 1

    src = bytearray(Path(sys.argv[1]).read_bytes())
    hook = Path(sys.argv[2]).read_bytes()

    tail_file_off = 0x1C560
    final_jmp_file_off = tail_file_off + 0x1A7
    mapped_hook_off = 0x220

    # Replace "jmp short 0x1c8" with "jmp short 0x220".
    src[final_jmp_file_off] = 0xEB
    src[final_jmp_file_off + 1] = 0x77

    src.extend(hook)
    patch_mz_size(src)
    Path(sys.argv[3]).write_bytes(src)
    print(f"wrote {sys.argv[3]} ({len(src)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
