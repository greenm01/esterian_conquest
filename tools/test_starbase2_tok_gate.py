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


def run_ecmaint(target: Path) -> str:
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
    if (target / "ERRORS.TXT").exists():
        return (target / "ERRORS.TXT").read_text(errors="ignore").splitlines()[0]
    return "OK"


def run_case(name: str, tok_name: str | None) -> None:
    target = Path("/tmp") / name
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(SRC, target)
    shutil.copy2(ECMAINT, target / "ECMAINT.EXE")

    player = bytearray((target / "PLAYER.DAT").read_bytes())
    player[0x44:0x48] = bytes([0x02, 0x00, 0x02, 0x00])
    (target / "PLAYER.DAT").write_bytes(player)
    (target / "BASES.DAT").write_bytes(build_two_base_file())

    if tok_name is not None:
        (target / tok_name).write_bytes(b"")

    first = run_ecmaint(target)
    second = run_ecmaint(target)
    bases = (target / "BASES.DAT").read_bytes()

    print(f"{name}: tok={tok_name or 'none'}")
    print("  pass1:", first)
    print("  pass2:", second)
    print("  bases length:", len(bases))
    print("  base1[0x00..0x08]:", bases[:9].hex())
    print("  base2[0x00..0x08]:", bases[35:44].hex())


if __name__ == "__main__":
    run_case("test-starbase2-no-tok", None)
    run_case("test-starbase2-main-tok", "MAIN.TOK")
    run_case("test-starbase2-player-tok", "PLAYER.TOK")
    run_case("test-starbase2-foo-tok", "FOO.TOK")
