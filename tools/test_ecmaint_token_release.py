import os
import shutil
import subprocess
import threading
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "fixtures" / "ecmaint-starbase-pre" / "v1.5"
ECMAINT = ROOT / "original" / "v1.5" / "ECMAINT.EXE"
ARTIFACT_DIR = ROOT / "artifacts" / "ecmaint-token-release"

BASE_RECORD_HEX = (
    "0100010001000001000000100d80000000000080000000000081000000000000100d01"
)


def build_two_base_file() -> bytes:
    base1 = bytearray.fromhex(BASE_RECORD_HEX)
    base2 = bytearray.fromhex(BASE_RECORD_HEX)
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


def prepare_scenario(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)
    shutil.copytree(SRC, path)
    shutil.copy2(ECMAINT, path / "ECMAINT.EXE")

    player = bytearray((path / "PLAYER.DAT").read_bytes())
    player[0x44:0x48] = bytes([0x02, 0x00, 0x02, 0x00])
    (path / "PLAYER.DAT").write_bytes(player)
    (path / "BASES.DAT").write_bytes(build_two_base_file())
    (path / "PLAYER.TOK").write_bytes(b"")


def run_case(delay_seconds: float) -> None:
    scenario = Path("/tmp") / f"ecmaint-token-release-{str(delay_seconds).replace('.', '_')}"
    prepare_scenario(scenario)
    log_path = ARTIFACT_DIR / f"delay-{str(delay_seconds).replace('.', '_')}.log"

    def remover() -> None:
        time.sleep(delay_seconds)
        token = scenario / "PLAYER.TOK"
        if token.exists():
            token.unlink()

    thread = threading.Thread(target=remover, daemon=True)
    thread.start()

    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    cmd = [
        "dosbox-x",
        "-defaultconf",
        "-nopromptfolder",
        "-nogui",
        "-nomenu",
        "-defaultdir",
        str(scenario),
        "-debug",
        "-log-int21",
        "-log-fileio",
        "-time-limit",
        "18",
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
        f"mount c {scenario}",
        "-c",
        "c:",
        "-c",
        "ECMAINT /R",
        "-c",
        "exit",
    ]
    with log_path.open("w") as handle:
        subprocess.run(cmd, stdout=handle, stderr=subprocess.STDOUT, env=env, check=False)

    error_line = "<none>"
    if (scenario / "ERRORS.TXT").exists():
        error_line = (scenario / "ERRORS.TXT").read_text(errors="ignore").splitlines()[0]

    print(f"delay={delay_seconds}s log={log_path}")
    print(f"  ERRORS.TXT: {error_line}")


def main() -> None:
    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)
    for delay in (4.0, 6.0, 8.0):
        run_case(delay)


if __name__ == "__main__":
    main()
