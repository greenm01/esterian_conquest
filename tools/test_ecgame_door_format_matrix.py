import json
import os
import re
import shutil
import time
from pathlib import Path

from ecgame_dropfiles import write_door_sys
from pexpect_argv import spawn_argv


TARGET_ROOT = Path("/tmp/ecgame-door-format-matrix")
ARTIFACT_DIR = Path("artifacts/ecgame-startup")


def write_legacy_door_sys(path: Path) -> None:
    text = """COM1:
19200
8
1
19200
Y
Y
Y
Y
Sysop First
Sysop Last
Orlando, FL
1
1
100
9000
1
2
90
90
90
90
90
90
"""
    path.write_bytes(text.replace("\n", "\r\n").encode("ascii"))


def prepare_variant(name: str) -> Path:
    target = TARGET_ROOT / name
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree("fixtures/ecutil-init/v1.5", target)
    shutil.copy2("original/v1.5/ECGAME.EXE", target)
    if name == "shared":
        write_door_sys(target / "DOOR.SYS", user_first_name="Sysop", user_last_name="HANNIBAL")
    elif name == "legacy":
        write_legacy_door_sys(target / "DOOR.SYS")
    else:
        raise ValueError(name)
    return target


def read_available(child) -> str:
    text = ""
    while True:
        try:
            text += child.read_nonblocking(size=4096, timeout=0.2)
        except Exception:
            break
    return text


def send(child, cmd_text: str, delay: float = 0.6) -> None:
    child.sendline(cmd_text)
    time.sleep(delay)


def capture_ev(child) -> list[str]:
    send(child, "EV AX BX CX DX SI DI BP SP CS DS ES SS", 0.5)
    text = read_available(child)
    ev_match = re.search(
        r"EV of 'AX BX CX DX SI DI BP SP CS DS ES SS' is:\s*LOG:\s*([0-9a-fA-F ]+)",
        text,
    )
    if not ev_match:
        raise RuntimeError(f"Unable to parse EV output:\n{text}")
    values = ev_match.group(1).split()
    if len(values) != 12:
        raise RuntimeError(f"Unexpected EV field count {len(values)} in:\n{text}")
    return values


def arm_breakpoints(child, ah_values: tuple[str, ...]) -> None:
    send(child, "BPDEL *", 0.5)
    for ah in ah_values:
        send(child, f"BPINT 21 {ah}", 0.3)


def run_variant(target: Path) -> dict[str, object]:
    cmd = [
        "dosbox-x",
        "-defaultconf",
        "-nopromptfolder",
        "-nogui",
        "-nomenu",
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
        "DEBUGBOX ECGAME.EXE",
    ]

    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    env["TERM"] = "dumb"

    child = spawn_argv(cmd, env=env, timeout=20, encoding="cp437")
    try:
        time.sleep(3)

        send(child, "BPINT 21 3D", 1.0)
        send(child, "RUN", 4.0)
        read_available(child)
        capture_ev(child)  # open Setup.dat

        arm_breakpoints(child, ("3F", "3E", "3D", "4C"))
        sequence: list[str] = []
        exit_code: str | None = None
        for _ in range(24):
            send(child, "RUN", 4.0)
            tail = read_available(child)
            values = capture_ev(child)
            ax = values[0].upper()
            sequence.append(ax)
            if ax.startswith("3D"):
                arm_breakpoints(child, ("3F", "3E", "3D", "4C"))
            if ax.startswith("4C"):
                exit_code = values[1].upper()
                return {
                    "door_len": (target / "DOOR.SYS").stat().st_size,
                    "sequence": sequence,
                    "exit_code": exit_code,
                    "tail": tail[-200:],
                }

        return {
            "door_len": (target / "DOOR.SYS").stat().st_size,
            "sequence": sequence,
            "exit_code": exit_code,
        }
    finally:
        try:
            send(child, "EXIT", 0.2)
        except Exception:
            pass
        if child.isalive():
            child.close(force=True)


def main() -> None:
    if TARGET_ROOT.exists():
        shutil.rmtree(TARGET_ROOT)
    TARGET_ROOT.mkdir(parents=True)
    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)

    results = {}
    for name in ("shared", "legacy"):
        results[name] = run_variant(prepare_variant(name))

    artifact_path = ARTIFACT_DIR / "door-format-matrix.json"
    artifact_path.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
