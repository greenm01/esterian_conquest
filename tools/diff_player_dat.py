#!/usr/bin/env python3
from __future__ import annotations

import sys
from pathlib import Path


PLAYER_RECORD_SIZE = 110


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: python3 tools/diff_player_dat.py <before> <after>", file=sys.stderr)
        return 2

    before_path = Path(sys.argv[1])
    after_path = Path(sys.argv[2])
    before = before_path.read_bytes()
    after = after_path.read_bytes()

    if len(before) != len(after):
        print(
            f"size mismatch: before={len(before)} bytes after={len(after)} bytes",
            file=sys.stderr,
        )
        return 1

    if len(before) % PLAYER_RECORD_SIZE != 0:
        print(
            f"unexpected PLAYER.DAT size {len(before)}; not divisible by {PLAYER_RECORD_SIZE}",
            file=sys.stderr,
        )
        return 1

    changed = False
    for absolute_offset, (old, new) in enumerate(zip(before, after)):
        if old == new:
            continue
        changed = True
        record_index = absolute_offset // PLAYER_RECORD_SIZE
        record_offset = absolute_offset % PLAYER_RECORD_SIZE
        print(
            f"record={record_index + 1} abs={absolute_offset:#04x} "
            f"rec_off={record_offset:#04x}: {old:#04x} -> {new:#04x}"
        )

    if not changed:
        print("no PLAYER.DAT byte diffs")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
