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


def patch_player_starbase_fields(player_path: Path, count: int) -> None:
    data = bytearray(player_path.read_bytes())
    data[0x44:0x48] = bytes([count, 0x00, count, 0x00])
    player_path.write_bytes(data)


def build_two_base_file(base2_id: int) -> bytes:
    base1 = bytearray.fromhex(BASE_RECORD_HEX)
    base2 = bytearray.fromhex(BASE_RECORD_HEX)

    # Link base 1 -> base 2.
    base1[0x08] = 0x02

    # Candidate second base at (4,13).
    base2[0x00] = 0x02
    base2[0x04] = base2_id
    base2[0x07] = 0x01
    base2[0x0B] = 0x04
    base2[0x0C] = 0x0D
    base2[0x1E] = 0x04
    base2[0x1F] = 0x0D
    return bytes(base1) + bytes(base2)


def run_case(name: str, base2_id: int) -> None:
    target = Path("/tmp") / name
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(SRC, target)
    shutil.copy2(ECMAINT, target / "ECMAINT.EXE")

    patch_player_starbase_fields(target / "PLAYER.DAT", 2)
    (target / "BASES.DAT").write_bytes(build_two_base_file(base2_id))

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
    player = (target / "PLAYER.DAT").read_bytes()
    bases = (target / "BASES.DAT").read_bytes()

    print(f"{name}: base2[0x04]={base2_id}")
    print("  errors:", "yes" if errors else "no")
    if errors:
        print("  first error:", errors.splitlines()[0])
    print("  player[0x44:0x48]:", player[0x44:0x48].hex())
    print("  bases length:", len(bases))
    for i in range(len(bases) // 35):
        rec = bases[i * 35 : (i + 1) * 35]
        print(f"  base{i+1}[0x00..0x08]:", rec[:9].hex())


if __name__ == "__main__":
    run_case("test-starbase2-baseid2", 2)
    run_case("test-starbase2-baseid1", 1)
