import os
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "fixtures" / "ecmaint-starbase-pre" / "v1.5"
ECMAINT = ROOT / "original" / "v1.5" / "ECMAINT.EXE"


def run_case(name: str, player48: int) -> None:
    target = Path("/tmp") / name
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(SRC, target)
    shutil.copy2(ECMAINT, target / "ECMAINT.EXE")

    player = bytearray((target / "PLAYER.DAT").read_bytes())
    player[0x48:0x4A] = bytes([player48, 0x00])
    (target / "PLAYER.DAT").write_bytes(player)

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
    player_post = (target / "PLAYER.DAT").read_bytes()

    print(f"{name}: player[0x48]={player48}")
    print("  errors:", "yes" if errors else "no")
    if errors:
        print("  first error:", errors.splitlines()[0])
    print("  player[0x44:0x4a]:", player_post[0x44:0x4A].hex())


if __name__ == "__main__":
    for value in (0, 1, 2):
        run_case(f"test-player48-{value}", value)
