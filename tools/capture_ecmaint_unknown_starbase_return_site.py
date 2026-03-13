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
TARGET = Path("/tmp/ecmaint-debug-unknown-starbase-return")
ARTIFACT_DIR = ROOT / "artifacts" / "ecmaint-kind2-debug" / "unknown-starbase-return-site"

BREAKPOINT = "2895:27ac"
TARGET_CS = 0x2895
TARGET_EIP = 0x27ac


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


def memdump(child, seg: int, off: int, length: int) -> str:
    send(child, f"MEMDUMPBIN {seg:04X}:{off:04X} {length}", 0.8)
    read_available(child)
    return subprocess.check_output(
        ["xxd", "-g", "1", "-l", str(length), str(TARGET / "MEMDUMP.BIN")],
        text=True,
    )


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
        send(child, "BPINT 21 3D", 0.2)
        send(child, "RUN", 3.0)
        transcript.append(read_available(child))
        send(child, "BPDEL *", 0.2)
        send(child, f"BP {BREAKPOINT}", 0.2)
        send(child, "BPINT 21 4C", 0.2)

        hit_ev = None
        hit_regs = None
        hits = []
        for _ in range(30):
            send(child, "RUN", 2.0)
            transcript.append(read_available(child))
            ev_block, regs = capture_ev(child)
            hits.append(ev_block)
            if regs["CS"] == TARGET_CS and regs["EIP"] == TARGET_EIP:
                hit_ev = ev_block
                hit_regs = regs
                break
            if regs["AX"] >> 8 == 0x4C:
                break

        if hit_ev is None or hit_regs is None:
            raise RuntimeError(f"Did not hit {BREAKPOINT}. EV hits: {hits}")

        ds = hit_regs["DS"]
        ss = hit_regs["SS"]
        bp = hit_regs["BP"]
        scratch_hex = memdump(child, ds, 0x3502, 0x30)
        mode_hex = memdump(child, ds, 0x0630, 0x10)
        stack_hex = memdump(child, ss, bp, 0x40)

        (ARTIFACT_DIR / "registers.txt").write_text(hit_ev + "\n")
        (ARTIFACT_DIR / "scratch_3502.txt").write_text(scratch_hex)
        (ARTIFACT_DIR / "mode_0630.txt").write_text(mode_hex)
        (ARTIFACT_DIR / "stack_bp.txt").write_text(stack_hex)
        (ARTIFACT_DIR / "ev_hits.txt").write_text("\n".join(hits) + "\n")

        summary = []
        summary.append("Unknown-starbase return-site capture")
        summary.append("")
        summary.append(hit_ev)
        summary.append("")
        summary.append(f"DS={ds:04X} SS={ss:04X} BP={bp:04X}")
        summary.append("")
        summary.append("DS:3502")
        summary.append(scratch_hex)
        summary.append("")
        summary.append("DS:0630")
        summary.append(mode_hex)
        summary.append("")
        summary.append("SS:BP")
        summary.append(stack_hex)
        (ARTIFACT_DIR / "summary.txt").write_text("\n".join(summary) + "\n")

        child.sendcontrol("c")
        if child.before:
            transcript.append(child.before)
    finally:
        clean = [part for part in transcript if isinstance(part, str)]
        (ARTIFACT_DIR / "session.log").write_text("".join(clean))
        child.close(force=True)

    print(ARTIFACT_DIR)


if __name__ == "__main__":
    main()
