import os
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "fixtures" / "ecmaint-starbase-pre" / "v1.5"
ECMAINT = ROOT / "original" / "v1.5" / "ECMAINT.EXE"
BASE_RECORD_HEX = (
    "0100010001000001000000100d80000000000080000000000081000000000000100d01"
)


def run_case(name: str, link_lo: int, link_hi: int) -> None:
    target = Path("/tmp") / name
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(SRC, target)
    shutil.copy2(ECMAINT, target / "ECMAINT.EXE")

    player = bytearray((target / "PLAYER.DAT").read_bytes())
    player[0x44:0x48] = bytes([0x02, 0x00, 0x02, 0x00])
    (target / "PLAYER.DAT").write_bytes(player)

    base1 = bytearray.fromhex(BASE_RECORD_HEX)
    base2 = bytearray.fromhex(BASE_RECORD_HEX)
    base1[0x08] = 0x02

    # Accepted duplicate-base setup, except for the secondary base word at 0x05..0x06.
    base2[0x00] = 0x02
    base2[0x02] = 0x01
    base2[0x04] = 0x01
    base2[0x05] = link_lo
    base2[0x06] = link_hi
    base2[0x07] = 0x01
    base2[0x0B] = 0x04
    base2[0x0C] = 0x0D
    base2[0x1E] = 0x04
    base2[0x1F] = 0x0D
    (target / "BASES.DAT").write_bytes(bytes(base1) + bytes(base2))

    cmd = [
        "dosbox-x",
        "-defaultconf",
        "-nopromptfolder",
        "-defaultdir",
        str(target),
        "-set",
        "dosv=off",
        "-set",
        "machine=vgaonly",
        "-set",
        "core=normal",
        "-set",
        "cputype=386_prefetch",
        "-set",
        "cycles=fixed 3000",
        "-set",
        "xms=false",
        "-set",
        "ems=false",
        "-set",
        "umb=false",
        "-set",
        "output=surface",
        "-time-limit",
        "12",
        "-c",
        f"mount c {target}",
        "-c",
        "c:",
        "-c",
        "ECMAINT /R",
        "-c",
        "exit",
    ]
    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)

    errors = (target / "ERRORS.TXT").read_text(errors="ignore") if (target / "ERRORS.TXT").exists() else ""
    player_post = (target / "PLAYER.DAT").read_bytes()
    bases_len = (target / "BASES.DAT").stat().st_size if (target / "BASES.DAT").exists() else 0

    print(f"{name}: base2[0x05:0x07]={bytes([link_lo, link_hi]).hex()}")
    print("  errors:", "yes" if errors else "no")
    if errors:
        print("  first error:", errors.splitlines()[0])
    print("  player[0x44:0x48]:", player_post[0x44:0x48].hex())
    print("  bases length:", bases_len)


if __name__ == "__main__":
    for lo, hi in (
        (0x00, 0x00),
        (0x01, 0x00),
        (0x00, 0x01),
        (0x01, 0x01),
        (0x02, 0x00),
    ):
        run_case(f"test-starbase-link-{lo:02x}{hi:02x}", lo, hi)
