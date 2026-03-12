import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

import pexpect


ROOT = Path(__file__).resolve().parents[1]
TARGET = Path("/tmp/ecmaint-debug-ipbm")

LIVE_BREAKPOINTS = [
    "2814:6870",  # first summary write from DS:3538
    "2814:69cd",  # second-branch summary write from DS:3538
    "2814:6a4b",  # second-branch DS:353D -> summary+0x06
]


def run_black_box_sanity() -> None:
    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    subprocess.run(
        [
            "dosbox-x",
            "-defaultconf",
            "-nopromptfolder",
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
            "ECMAINT /R",
            "-c",
            "exit",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        env=env,
        check=False,
    )


def capture() -> None:
    suffix = sys.argv[1] if len(sys.argv) > 1 else "baseline"
    artifact_dir = ROOT / "artifacts" / "ecmaint-ipbm-debug" / suffix
    log_path = artifact_dir / "session.log"
    reg_path = artifact_dir / "registers.txt"
    hex_path = artifact_dir / "scratch-hex.txt"

    if artifact_dir.exists():
        shutil.rmtree(artifact_dir)
    artifact_dir.mkdir(parents=True)

    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    env["TERM"] = "dumb"

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
        "DEBUGBOX ECMAINT /R",
    ]

    child = pexpect.spawn(" ".join(cmd), env=env, encoding="utf-8", timeout=20)
    transcript = []

    def send(cmd_text: str, delay: float = 0.4) -> None:
        child.sendline(cmd_text)
        time.sleep(delay)

    try:
        time.sleep(2)
        send("BPINT 21 3D")
        send("RUN", 1.5)
        child.expect(r"3374:00001880", timeout=10)
        transcript.append(child.before + child.after)

        for bp in LIVE_BREAKPOINTS:
            send(f"BP {bp}")
        send("BPLIST")
        send("BPDEL 03")  # remove BPINT after code breakpoints are armed
        send("RUN", 1.5)

        child.expect(r"2814:6870|3159:00000820|2814:69cd|2814:6a4b", timeout=15)
        transcript.append(child.before + child.after)

        send("EV CS EIP DS ES SS SP BP AX BX CX DX SI DI", 0.5)
        child.expect(r"LOG: EV of 'CS EIP DS ES SS SP BP AX BX CX DX SI DI' is:\s*[\r\n]+LOG:\s+([0-9A-Fa-f ]+)", timeout=5)
        ev_block = child.match.group(1).strip()
        transcript.append(child.before + child.after)
        reg_path.write_text(ev_block + "\n")

        parts = ev_block.split()
        ds = parts[2]
        send(f"MEMDUMPBIN {ds}:3538 20", 0.8)
        transcript.append(child.before)

        # Leave debugger cleanly.
        child.sendcontrol("c")
        child.expect(r"y/n:", timeout=5)
        child.sendline("y")
        child.expect(pexpect.EOF, timeout=5)
        transcript.append(child.before)
    finally:
        log_path.write_text("".join(transcript))
        child.close(force=True)

    memdump = TARGET / "MEMDUMP.BIN"
    if memdump.exists():
        hexdump = subprocess.check_output(["xxd", "-g", "1", "-l", "32", str(memdump)], text=True)
        hex_path.write_text(hexdump)
    return artifact_dir


def main() -> None:
    run_black_box_sanity()
    errors = TARGET / "ERRORS.TXT"
    if errors.exists() and errors.read_text(errors="ignore").strip():
        print("Sanity run produced ERRORS.TXT; debug case is not valid enough for scratch capture")
        return

    artifact_dir = capture()
    print(artifact_dir)
    print("Captured IPBM scratch breakpoint artifacts")


if __name__ == "__main__":
    main()
