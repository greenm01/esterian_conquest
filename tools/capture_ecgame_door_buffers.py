import os
import re
import shutil
import time
from pathlib import Path

from ecgame_dropfiles import write_door_sys
from pexpect_argv import spawn_argv


TARGET = Path("/tmp/ecgame-door-buffers")
ARTIFACT_DIR = Path("artifacts/ecgame-startup")


def prepare_target() -> None:
    if TARGET.exists():
        shutil.rmtree(TARGET)
    shutil.copytree("fixtures/ecutil-init/v1.5", TARGET)
    shutil.copy2("original/v1.5/ECGAME.EXE", TARGET)
    write_door_sys(
        TARGET / "DOOR.SYS",
        user_first_name="Sysop",
        user_last_name="HANNIBAL",
    )


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


def dump_buffer(child, ds: str, offset: str, name: str) -> bytes:
    send(child, f"MEMDUMPBIN {ds}:{offset} 80", 1.0)
    read_available(child)
    data = (TARGET / "MEMDUMP.BIN").read_bytes()[:0x80]
    (ARTIFACT_DIR / name).write_bytes(data)
    return data


def decode_ascii_prefix(data: bytes) -> str:
    prefix = []
    for b in data:
        if 32 <= b <= 126 or b in (0x0D, 0x0A):
            prefix.append(chr(b))
        else:
            break
    return "".join(prefix)


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
        run_stop(child)  # open Setup.dat

        arm_breakpoints(child, ("3F", "3E", "3D", "4C"))
        run_stop(child)  # read Setup.dat
        run_stop(child)  # close Setup.dat
        run_stop(child)  # open DOOR.SYS

        arm_breakpoints(child, ("3F", "3E", "4C"))
        run_stop(child)  # first 3F break, before the first DOOR.SYS read executes

        second_read_pre = run_stop(child)  # second 3F break, first read has completed
        ds = second_read_pre[9].upper()
        dx = second_read_pre[3].upper()
        first_bytes = dump_buffer(child, ds, dx, "door-buffer-first.bin")

        close_stop = run_stop(child)  # close after the second DOOR.SYS read returns
        ds2 = close_stop[9].upper()
        second_bytes = dump_buffer(child, ds2, "40BC", "door-buffer-second.bin")

        door_bytes = (TARGET / "DOOR.SYS").read_bytes()
        summary = [
            f"door_len={len(door_bytes)}",
            f"first_buffer_prefix_match={first_bytes[:0x80] == door_bytes[:0x80]}",
            f"first_buffer_ascii_prefix={decode_ascii_prefix(first_bytes)!r}",
            f"second_buffer_ascii_prefix={decode_ascii_prefix(second_bytes)!r}",
            f"first_buffer_head={first_bytes[:32].hex()}",
            f"second_buffer_head={second_bytes[:32].hex()}",
            f"second_read_pre_ax={second_read_pre[0]}",
            f"close_stop_ax={close_stop[0]}",
        ]
        (ARTIFACT_DIR / "door-buffer-summary.txt").write_text(
            "\n".join(summary) + "\n",
            encoding="utf-8",
        )
        print("\n".join(summary))
    finally:
        try:
            send(child, "EXIT", 0.2)
        except Exception:
            pass
        if child.isalive():
            child.close(force=True)


if __name__ == "__main__":
    main()
