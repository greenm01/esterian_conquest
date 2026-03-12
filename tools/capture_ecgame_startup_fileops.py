import os
import re
import shutil
import time
from pathlib import Path

from ecgame_dropfiles import write_chain_txt
from pexpect_argv import spawn_argv


TARGET = Path("/tmp/ecgame-startup-fileops")
ARTIFACT_DIR = Path("artifacts/ecgame-startup")


def prepare_target() -> None:
    if TARGET.exists():
        shutil.rmtree(TARGET)
    shutil.copytree("fixtures/ecutil-init/v1.5", TARGET)
    shutil.copy2("original/v1.5/ECGAME.EXE", TARGET)
    write_chain_txt(TARGET / "CHAIN.TXT")


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


def decode_string_dump(path: Path) -> str:
    data = path.read_bytes()
    zero = data.find(b"\x00")
    if zero < 0:
        zero = len(data)
    return data[:zero].decode("cp437", errors="replace")


def arm_breakpoints(child, ah_values: tuple[str, ...]) -> None:
    send(child, "BPDEL *", 0.5)
    for ah in ah_values:
        send(child, f"BPINT 21 {ah}", 0.4)


def run_stop(child) -> tuple[str, list[str]]:
    send(child, "RUN", 4.0)
    screen = read_available(child)
    values = capture_ev(child)
    return screen, values


def classify_stop(values: list[str], opened_name: str | None = None) -> str:
    ax = values[0].lower()
    bx = values[1].upper()
    cx = values[2].upper()
    dx = values[3].upper()
    if ax.startswith("3d"):
        return f"open {opened_name or '<unknown>'} mode={ax[2:]} ds:dx={dx}"
    if ax.startswith("3f"):
        return f"read handle={bx} count=0x{cx} buffer_ds:dx={dx}"
    if ax.startswith("3e"):
        return f"close handle={bx}"
    if ax.startswith("4c"):
        return f"exit code=0x{bx}"
    return f"ax={values[0]} bx={bx} cx={cx} dx={dx}"


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
    transcript_parts: list[str] = []
    summary_lines: list[str] = []
    try:
        time.sleep(3)

        send(child, "BPINT 21 3D", 1.0)
        screen, values = run_stop(child)
        transcript_parts.append(screen)
        ds = values[9].upper()
        dx = values[3].upper()
        send(child, f"MEMDUMPBIN {ds}:{dx} 100", 1.0)
        transcript_parts.append(read_available(child))
        first_name = decode_string_dump(TARGET / "MEMDUMP.BIN")
        summary_lines.append(f"1. {classify_stop(values, first_name)}")

        arm_breakpoints(child, ("3F", "3E", "3D", "4C"))

        second_open_name: str | None = None
        for stop_idx in range(2, 8):
            screen, values = run_stop(child)
            transcript_parts.append(screen)
            ax = values[0].lower()
            opened_name = None
            if ax.startswith("3d"):
                ds = values[9].upper()
                dx = values[3].upper()
                send(child, f"MEMDUMPBIN {ds}:{dx} 100", 1.0)
                transcript_parts.append(read_available(child))
                opened_name = decode_string_dump(TARGET / "MEMDUMP.BIN")
                if second_open_name is None:
                    second_open_name = opened_name
            summary_lines.append(f"{stop_idx}. {classify_stop(values, opened_name)}")

            if ax.startswith("3d"):
                arm_breakpoints(child, ("3F", "3E", "3D", "4C"))

        (ARTIFACT_DIR / "startup-fileops.txt").write_text(
            "\n".join(summary_lines) + "\n",
            encoding="utf-8",
        )
        (ARTIFACT_DIR / "startup-fileops-transcript.txt").write_text(
            "".join(transcript_parts),
            encoding="utf-8",
            errors="replace",
        )
        print("\n".join(summary_lines))
    finally:
        try:
            send(child, "EXIT", 0.2)
        except Exception:
            pass
        if child.isalive():
            child.close(force=True)


if __name__ == "__main__":
    main()
