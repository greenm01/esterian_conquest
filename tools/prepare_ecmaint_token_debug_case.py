#!/usr/bin/env python3

from pathlib import Path
import shutil


ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "fixtures" / "ecmaint-starbase-pre" / "v1.5"
ECMAINT = ROOT / "original" / "v1.5" / "ECMAINT.EXE"
TARGET = Path("/tmp/ecmaint-debug-token")

BASE_RECORD_HEX = (
    "0100010001000001000000100d80000000000080000000000081000000000000100d01"
)


def build_two_base_file() -> bytes:
    base1 = bytearray.fromhex(BASE_RECORD_HEX)
    base2 = bytearray.fromhex(BASE_RECORD_HEX)

    # Raw two-base attempt: Starbase 1 at (16,13), Starbase 2 at (4,13).
    base1[0x08] = 0x02
    base2[0x00] = 0x02
    base2[0x02] = 0x01
    base2[0x04] = 0x02
    base2[0x05] = 0x01
    base2[0x07] = 0x01
    base2[0x0B] = 0x04
    base2[0x0C] = 0x0D
    base2[0x1E] = 0x04
    base2[0x1F] = 0x0D
    return bytes(base1) + bytes(base2)


def main() -> None:
    if TARGET.exists():
        shutil.rmtree(TARGET)
    shutil.copytree(SRC, TARGET)
    shutil.copy2(ECMAINT, TARGET / "ECMAINT.EXE")

    player = bytearray((TARGET / "PLAYER.DAT").read_bytes())
    player[0x44:0x48] = bytes([0x02, 0x00, 0x02, 0x00])
    (TARGET / "PLAYER.DAT").write_bytes(player)
    (TARGET / "BASES.DAT").write_bytes(build_two_base_file())
    (TARGET / "PLAYER.TOK").write_bytes(b"")

    print(TARGET)
    print("Prepared raw two-base + PLAYER.TOK debug case")


if __name__ == "__main__":
    main()
