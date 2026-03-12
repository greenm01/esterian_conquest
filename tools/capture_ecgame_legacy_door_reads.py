import json
import os
import re
import shutil
import time
from pathlib import Path

from pexpect_argv import spawn_argv


TARGET = Path("/tmp/ecgame-legacy-door-reads")
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


def run_stop(child) -> list[str]:
    send(child, "RUN", 4.0)
    read_available(child)
    return capture_ev(child)


def dump_buffer(child, ds: str, offset: str, size_hex: str = "80") -> bytes:
    send(child, f"MEMDUMPBIN {ds}:{offset} {size_hex}", 1.0)
    read_available(child)
    return (TARGET / "MEMDUMP.BIN").read_bytes()


def printable_prefix(data: bytes) -> str:
    out = []
    for b in data:
        if b in (0x0D, 0x0A) or 32 <= b <= 126:
            out.append(chr(b))
        else:
            break
    return "".join(out)


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
        time.sleep(3)

        send(child, "BPINT 21 3D", 1.0)
        run_stop(child)  # Setup.dat open

        arm_breakpoints(child, ("3F", "3E", "3D", "4C"))
        run_stop(child)  # Setup.dat read
        run_stop(child)  # Setup.dat close
        run_stop(child)  # DOOR.SYS open

        reads: list[dict[str, object]] = []
        door_bytes = (TARGET / "DOOR.SYS").read_bytes()
        cumulative = 0

        arm_breakpoints(child, ("3F", "3E", "4C"))
        for _ in range(24):
            values = run_stop(child)
            ax = values[0].upper()
            if ax.startswith("3E") or ax.startswith("4C"):
                break
            if not ax.startswith("3F"):
                continue

            read_len = int(ax[2:], 16)
            ds = values[9].upper()
            dx = values[3].upper()
            data = dump_buffer(child, ds, dx)
            expected = door_bytes[cumulative : cumulative + min(read_len, len(door_bytes) - cumulative)]
            buffer_prefix = data[: len(expected)]
            reads.append(
                {
                    "ax": ax,
                    "read_len": read_len,
                    "buffer_offset": dx,
                    "expected_offset": cumulative,
                    "prefix_match": buffer_prefix == expected,
                    "ascii_prefix": printable_prefix(data[: max(read_len, 16)]),
                    "head_hex": data[:32].hex(),
                }
            )
            cumulative += max(read_len, 0)

        artifact_path = ARTIFACT_DIR / "legacy-door-reads.json"
        artifact_path.write_text(json.dumps(reads, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(reads, indent=2))
    finally:
        try:
            send(child, "EXIT", 0.2)
        except Exception:
            pass
        if child.isalive():
            child.close(force=True)


if __name__ == "__main__":
    main()
