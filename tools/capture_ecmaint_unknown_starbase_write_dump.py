import os
import re
import shutil
import subprocess
import time
from pathlib import Path

from pexpect_argv import spawn_argv


ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "fixtures" / "ecmaint-starbase-pre" / "v1.5"
ECMAINT = ROOT / "original" / "v1.5" / "ECMAINT.EXE"
TARGET = Path("/tmp/ecmaint-debug-unknown-starbase-write")
ARTIFACT_DIR = ROOT / "artifacts" / "ecmaint-kind2-debug" / "unknown-starbase-write"


def run_sanity() -> None:
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
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
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


def parse_ev_registers(block: str) -> dict[str, int]:
    parts = [int(part, 16) for part in block.split()]
    names = ["CS", "EIP", "DS", "ES", "SS", "SP", "BP", "AX", "BX", "CX", "DX", "SI", "DI"]
    return dict(zip(names, parts, strict=True))


def capture_ev(child) -> tuple[str, dict[str, int]]:
    send(child, "EV CS EIP DS ES SS SP BP AX BX CX DX SI DI", 0.5)
    text = read_available(child)
    ev_match = re.search(
        r"EV of 'CS EIP DS ES SS SP BP AX BX CX DX SI DI' is:\s*LOG:\s*([0-9a-fA-F ]+)",
        text,
    )
    if not ev_match:
        raise RuntimeError(f"Unable to parse EV output:\n{text}")
    ev_block = ev_match.group(1).strip()
    return ev_block, parse_ev_registers(ev_block)


def main() -> None:
    if TARGET.exists():
        shutil.rmtree(TARGET)
    shutil.copytree(SRC, TARGET)
    if not (TARGET / "ECMAINT.EXE").exists():
        shutil.copy2(ECMAINT, TARGET / "ECMAINT.EXE")

    fleets = bytearray((TARGET / "FLEETS.DAT").read_bytes())
    fleets[0x23] = 0x00
    (TARGET / "FLEETS.DAT").write_bytes(fleets)

    run_sanity()

    if ARTIFACT_DIR.exists():
        shutil.rmtree(ARTIFACT_DIR)
    ARTIFACT_DIR.mkdir(parents=True)

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

    child = spawn_argv(cmd, env=env, timeout=30, encoding="utf-8")
    transcript = []

    try:
        time.sleep(2)
        send(child, "BPDEL *", 0.5)
        send(child, "BPINT 21 40", 0.2)

        hit_ev = None
        hits = []
        for _ in range(20):
            send(child, "RUN", 2.0)
            transcript.append(read_available(child))
            ev_block, regs = capture_ev(child)
            hits.append(ev_block)
            if regs["AX"] >> 8 == 0x40:
                hit_ev = ev_block
                break
            if regs["AX"] >> 8 == 0x4C:
                break

        if hit_ev is None:
            raise RuntimeError(f"Did not hit INT 21h/AH=40 write stop. EV hits: {hits}")

        send(child, "DOS MCBS", 0.6)
        mcbs = read_available(child)
        send(child, "MEMDUMPBIN 0814:0000 97EB0", 1.0)
        transcript.append(read_available(child))

        dump_path = TARGET / "MEMDUMP.BIN"
        if dump_path.exists():
            shutil.copy2(dump_path, ARTIFACT_DIR / "MEMDUMP.BIN")

        (ARTIFACT_DIR / "registers.txt").write_text(hit_ev + "\n")
        (ARTIFACT_DIR / "mcbs.txt").write_text(mcbs)
        (ARTIFACT_DIR / "ev_hits.txt").write_text("\n".join(hits) + "\n")
        if (TARGET / "ERRORS.TXT").exists():
            shutil.copy2(TARGET / "ERRORS.TXT", ARTIFACT_DIR / "ERRORS.TXT")

        summary = []
        summary.append("Unknown-starbase write-stop dump")
        summary.append("")
        summary.append(hit_ev)
        summary.append("")
        summary.append("MCBS:")
        summary.append(mcbs.strip())
        summary.append("")
        if (TARGET / "ERRORS.TXT").exists():
            summary.append("ERRORS.TXT:")
            summary.append((TARGET / "ERRORS.TXT").read_text(errors="ignore").strip())
        (ARTIFACT_DIR / "summary.txt").write_text("\n".join(summary) + "\n")

        child.sendcontrol("c")
        child.expect(r"y/n:", timeout=5)
        child.sendline("y")
        child.expect_exact("Killed", timeout=5)
        transcript.append(child.before)
    finally:
        (ARTIFACT_DIR / "session.log").write_text("".join(transcript))
        child.close(force=True)

    print(ARTIFACT_DIR)


if __name__ == "__main__":
    main()
