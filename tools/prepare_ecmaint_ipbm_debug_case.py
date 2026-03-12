import shutil
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "fixtures" / "ecmaint-starbase-pre" / "v1.5"
ECMAINT = ROOT / "original" / "v1.5" / "ECMAINT.EXE"
TARGET = Path("/tmp/ecmaint-debug-ipbm")


def main() -> None:
    mutate_offset = None
    mutate_value = None
    if len(sys.argv) == 3:
        mutate_offset = int(sys.argv[1], 0)
        mutate_value = int(sys.argv[2], 0)

    if TARGET.exists():
        shutil.rmtree(TARGET)
    shutil.copytree(SRC, TARGET)
    shutil.copy2(ECMAINT, TARGET / "ECMAINT.EXE")

    player = bytearray((TARGET / "PLAYER.DAT").read_bytes())
    player[0x48:0x4A] = b"\x01\x00"
    (TARGET / "PLAYER.DAT").write_bytes(player)

    # Use a known-valid single-record baseline first. The record can be mutated
    # later once the normalized scratch layout is captured reliably.
    ipbm = bytearray(0x20)
    if mutate_offset is not None:
        ipbm[mutate_offset] = mutate_value
    (TARGET / "IPBM.DAT").write_bytes(ipbm)

    print(TARGET)
    if mutate_offset is None:
        print("Prepared PLAYER[0x48]=1 IPBM debug case with zeroed 0x20-byte IPBM.DAT")
    else:
        print(
            f"Prepared PLAYER[0x48]=1 IPBM debug case with IPBM[{mutate_offset:#04x}]={mutate_value:#04x}"
        )


if __name__ == "__main__":
    main()
