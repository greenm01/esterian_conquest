import os
import re
import shutil
import time
from pathlib import Path

from ecgame_dropfiles import write_chain_txt
from pexpect_argv import spawn_argv


TARGET = Path("/tmp/ecgame-chain-buffer")
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


def arm_breakpoints(child, ah_values: tuple[str, ...]) -> None:
    send(child, "BPDEL *", 0.5)
    for ah in ah_values:
        send(child, f"BPINT 21 {ah}", 0.4)


def run_stop(child) -> list[str]:
    send(child, "RUN", 4.0)
    read_available(child)
    return capture_ev(child)


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
        run_stop(child)  # first open: Setup.dat

        arm_breakpoints(child, ("3F", "3E", "3D", "4C"))
        run_stop(child)  # setup read
        run_stop(child)  # setup close
        run_stop(child)  # chain open

        arm_breakpoints(child, ("3F", "3E", "4C"))
        run_stop(child)  # chain read (pre-read)
        values = run_stop(child)  # chain close (post-read)

        ds = values[9].upper()
        send(child, f"MEMDUMPBIN {ds}:40BC 80", 1.0)
        read_available(child)

        buffer_bytes = (TARGET / "MEMDUMP.BIN").read_bytes()[:0x80]
        chain_bytes = (TARGET / "CHAIN.TXT").read_bytes()
        prefix_ok = buffer_bytes[: len(chain_bytes)] == chain_bytes
        tail_hex = buffer_bytes[len(chain_bytes) : len(chain_bytes) + 16].hex()

        summary = [
            f"close_stop_ax={values[0]}",
            f"chain_len={len(chain_bytes)}",
            f"buffer_len={len(buffer_bytes)}",
            f"prefix_match={prefix_ok}",
            f"matched_prefix_len={len(chain_bytes) if prefix_ok else 0}",
            f"tail_after_chain={tail_hex}",
        ]
        (ARTIFACT_DIR / "chain-buffer-summary.txt").write_text(
            "\n".join(summary) + "\n",
            encoding="utf-8",
        )
        (ARTIFACT_DIR / "chain-buffer-prefix.bin").write_bytes(buffer_bytes)
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
