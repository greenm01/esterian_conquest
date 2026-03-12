import json
import os
import re
import shutil
import time
from pathlib import Path

from pexpect_argv import spawn_argv


TARGET_ROOT = Path("/tmp/ecgame-legacy-tail-matrix")
ARTIFACT_DIR = Path("artifacts/ecgame-startup")


BASE_LINES = [
    "COM1:",
    "19200",
    "8",
    "1",
    "19200",
    "Y",
    "Y",
    "Y",
    "Y",
    "Sysop First",
    "Sysop Last",
    "Orlando, FL",
    "1",
    "1",
    "100",
    "9000",
    "1",
    "2",
]


def write_variant(path: Path, tail_count: int) -> int:
    lines = BASE_LINES + ["90"] * tail_count
    path.write_bytes(("\r\n".join(lines) + "\r\n").encode("ascii"))
    return len(lines)


def prepare_variant(name: str, tail_count: int) -> tuple[Path, int]:
    target = TARGET_ROOT / name
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree("fixtures/ecutil-init/v1.5", target)
    shutil.copy2("original/v1.5/ECGAME.EXE", target)
    line_count = write_variant(target / "DOOR.SYS", tail_count)
    return target, line_count


def read_available(child) -> str:
    text = ""
    while True:
        try:
            text += child.read_nonblocking(size=4096, timeout=0.2)
        except Exception:
            break
    return text


def send(child, cmd_text: str, delay: float = 0.5) -> None:
    child.sendline(cmd_text)
    time.sleep(delay)


def capture_ev(child) -> list[str]:
    send(child, "EV AX BX CX DX SI DI BP SP CS DS ES SS", 0.4)
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
    send(child, "BPDEL *", 0.3)
    for ah in ah_values:
        send(child, f"BPINT 21 {ah}", 0.2)


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
        time.sleep(2.5)

        send(child, "BPINT 21 3D", 0.8)
        send(child, "RUN", 4.0)
        read_available(child)
        capture_ev(child)  # open Setup.dat

        arm_breakpoints(child, ("3F", "3E", "3D", "4C"))
        for _ in range(3):
            send(child, "RUN", 3.5)
            read_available(child)
            capture_ev(child)  # setup read/close + door open

        arm_breakpoints(child, ("3F", "3E", "4C"))
        sequence: list[str] = []
        stable: list[tuple[str, int, int]] = []
        for _ in range(30):
            send(child, "RUN", 3.0)
            read_available(child)
            values = capture_ev(child)
            ax = values[0].upper()
            sequence.append(ax)

            if ax.startswith("3F") and values[6].upper() == "F6A4":
                ss = values[11].upper()
                bp = int(values[6], 16)
                send(child, f"MEMDUMPBIN {ss}:{bp + 0x0A:04X} 6", 0.8)
                read_available(child)
                data = (target / "MEMDUMP.BIN").read_bytes()
                limit = int.from_bytes(data[0:2], "little")
                current = int.from_bytes(data[2:4], "little")
                stable.append((ax, limit, current))

            if ax.startswith("4C"):
                break

        return {
            "sequence": sequence,
            "stable": stable,
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
    for tail_count in (4, 6, 8, 10, 12):
        target, line_count = prepare_variant(f"tail_{tail_count}", tail_count)
        results[f"tail_{tail_count}"] = {
            "lines": line_count,
            **run_variant(target),
        }

    artifact_path = ARTIFACT_DIR / "legacy-door-tail-matrix.json"
    artifact_path.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
