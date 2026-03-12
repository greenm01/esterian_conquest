import json
import os
import re
import shutil
import time
from pathlib import Path

from pexpect_argv import spawn_argv


TARGET = Path("/tmp/ecgame-legacy-handoff")
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


def prepare_target() -> None:
    if TARGET.exists():
        shutil.rmtree(TARGET)
    shutil.copytree("fixtures/ecutil-init/v1.5", TARGET)
    shutil.copy2("original/v1.5/ECGAME.EXE", TARGET)
    write_legacy_door_sys(TARGET / "DOOR.SYS")


def read_available(child) -> str:
    text = ""
    while True:
        try:
            text += child.read_nonblocking(size=4096, timeout=0.2)
        except Exception:
            break
    return text


def send(child, cmd_text: str, delay: float = 0.45) -> None:
    child.sendline(cmd_text)
    time.sleep(delay)


def capture_ev(child) -> list[str]:
    send(child, "EV AX BX CX DX SI DI BP SP CS DS ES SS", 0.35)
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


def run_stop(child) -> list[str]:
    send(child, "RUN", 2.5)
    read_available(child)
    return capture_ev(child)


def dump_mem(child, segment: str, offset_hex: str, size_hex: str, out_name: str) -> bytes:
    send(child, f"MEMDUMPBIN {segment}:{offset_hex} {size_hex}", 0.8)
    read_available(child)
    data = (TARGET / "MEMDUMP.BIN").read_bytes()
    (ARTIFACT_DIR / out_name).write_bytes(data)
    return data


def printable(data: bytes) -> str:
    chars: list[str] = []
    for b in data:
        if b in (0x0D, 0x0A) or 32 <= b <= 126:
            chars.append(chr(b))
        elif b == 0:
            break
        else:
            chars.append(".")
    return "".join(chars)


def sample_point(child, values: list[str], tag: str) -> dict[str, object]:
    ds = values[9].upper()
    ss = values[11].upper()
    si = values[4].upper()
    di = values[5].upper()
    bp = int(values[6], 16)
    frame_start = max(0, bp - 0x20)

    si_bytes = dump_mem(child, ds, si, "40", f"{tag}-si.bin")
    di_bytes = dump_mem(child, ds, di, "40", f"{tag}-di.bin")
    frame_bytes = dump_mem(child, ss, f"{frame_start:04X}", "60", f"{tag}-frame.bin")

    return {
        "ax": values[0].upper(),
        "si": si,
        "di": di,
        "bp": values[6].upper(),
        "sp": values[7].upper(),
        "cs": values[8].upper(),
        "ds": ds,
        "ss": ss,
        "frame_start": f"{frame_start:04X}",
        "si_ascii": printable(si_bytes[:64]),
        "di_ascii": printable(di_bytes[:64]),
        "si_head_hex": si_bytes[:32].hex(),
        "di_head_hex": di_bytes[:32].hex(),
        "frame_head_hex": frame_bytes[:48].hex(),
    }


def main() -> None:
    prepare_target()
    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)

    cmd = [
        "dosbox-x",
        "-defaultconf",
        "-nopromptfolder",
        "-nogui",
        "-nomenu",
        "-defaultdir",
        str(TARGET),
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
        f"mount c {TARGET}",
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
        time.sleep(2.2)
        send(child, "BPINT 21 3D", 0.7)
        send(child, "RUN", 3.6)
        read_available(child)
        capture_ev(child)  # setup open

        arm_breakpoints(child, ("3F", "3E", "3D", "4C"))
        for _ in range(3):
            run_stop(child)  # setup read/close + door open

        arm_breakpoints(child, ("3F", "3E", "4C"))
        wanted = {"3F10", "3FFF", "3F1A"}
        captured: dict[str, dict[str, object]] = {}
        while wanted:
            values = run_stop(child)
            ax = values[0].upper()
            if ax in wanted:
                captured[ax] = sample_point(child, values, ax.lower())
                wanted.remove(ax)
            if ax.startswith("4C"):
                break

        out_path = ARTIFACT_DIR / "legacy-door-handoff.json"
        out_path.write_text(json.dumps(captured, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(captured, indent=2))
    finally:
        try:
            send(child, "EXIT", 0.2)
        except Exception:
            pass
        if child.isalive():
            child.close(force=True)


if __name__ == "__main__":
    main()
