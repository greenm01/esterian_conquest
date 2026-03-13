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
TARGET = Path("/tmp/ecmaint-debug-kind2")
ARTIFACT_DIR = ROOT / "artifacts" / "ecmaint-kind2-debug"

BREAKPOINTS = {
    "base_decode_post_call": "0814:0403",
    "candidate_decode_post_call": "0814:0686",
}

TARGET_EIPS = {
    "base_decode_post_call": 0x0303,
    "candidate_decode_post_call": 0x0586,
}


def prepare_target() -> None:
    if TARGET.exists():
        shutil.rmtree(TARGET)
    shutil.copytree(SRC, TARGET)
    shutil.copy2(ECMAINT, TARGET / "ECMAINT.EXE")


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


def parse_ev_registers(block: str) -> dict[str, int]:
    parts = [int(part, 16) for part in block.split()]
    names = ["CS", "EIP", "DS", "ES", "SS", "SP", "BP", "AX", "BX", "CX", "DX", "SI", "DI"]
    return dict(zip(names, parts, strict=True))


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


def read_hexdump(path: Path, length: int) -> str:
    return subprocess.check_output(
        ["xxd", "-g", "1", "-l", str(length), str(path)],
        text=True,
    )


def capture_memory(child, registers: dict[str, int], linear_start_expr: str, length: int) -> str:
    if linear_start_expr == "DS:3558":
        seg = registers["DS"]
        off = 0x3558
    elif linear_start_expr == "LOCAL_CANDIDATE":
        seg = registers["SS"]
        off = (registers["BP"] - 0x84A) & 0xFFFF  # BP + 0xF7B6
    else:
        raise ValueError(linear_start_expr)

    send(child, f"MEMDUMPBIN {seg:04X}:{off:04X} {length}", 0.8)
    read_available(child)
    return read_hexdump(TARGET / "MEMDUMP.BIN", length)


def main() -> None:
    prepare_target()
    run_sanity()

    if (TARGET / "ERRORS.TXT").exists() and (TARGET / "ERRORS.TXT").read_text(errors="ignore").strip():
        raise SystemExit("Sanity run produced ERRORS.TXT")

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
        send(child, "BPINT 21 3D", 0.5)
        send(child, "RUN", 3.0)
        transcript.append(read_available(child))

        for bp in BREAKPOINTS.values():
            send(child, f"BP {bp}", 0.2)
        send(child, "BPDEL *", 0.2)
        for bp in BREAKPOINTS.values():
            send(child, f"BP {bp}", 0.2)
        send(child, "BPINT 21 4C", 0.2)

        base_ev = None
        base_hex = None
        cand_ev = None
        cand_hex = None
        hits = []
        non_candidate_stops_after_base = 0
        for _ in range(30):
            send(child, "RUN", 2.0)
            transcript.append(read_available(child))
            ev_block, regs = capture_ev(child)
            hits.append(ev_block)
            eip = regs["EIP"]
            if eip == TARGET_EIPS["base_decode_post_call"] and base_ev is None:
                base_ev = ev_block
                base_hex = capture_memory(child, regs, "DS:3558", 0x24)
                (ARTIFACT_DIR / "base_decode_registers.txt").write_text(base_ev + "\n")
                (ARTIFACT_DIR / "base_decode_3558.txt").write_text(base_hex)
            elif eip == TARGET_EIPS["candidate_decode_post_call"] and cand_ev is None:
                cand_ev = ev_block
                cand_hex = capture_memory(child, regs, "LOCAL_CANDIDATE", 0x30)
                (ARTIFACT_DIR / "candidate_decode_registers.txt").write_text(cand_ev + "\n")
                (ARTIFACT_DIR / "candidate_decode_local.txt").write_text(cand_hex)
            elif regs["AX"] >> 8 == 0x4C:
                break
            elif base_ev is not None and cand_ev is None:
                non_candidate_stops_after_base += 1
                if non_candidate_stops_after_base >= 3:
                    break
            if base_ev is not None and cand_ev is not None:
                break

        if base_ev is None or base_hex is None:
            raise RuntimeError(f"Did not hit base decode breakpoint. EV hits: {hits}")
        (ARTIFACT_DIR / "ev_hits.txt").write_text("\n".join(hits) + "\n")

        child.sendcontrol("c")
        child.expect(r"y/n:", timeout=5)
        child.sendline("y")
        child.expect_exact("Killed", timeout=5)
        transcript.append(child.before)
    finally:
        (ARTIFACT_DIR / "session.log").write_text("".join(transcript))
        child.close(force=True)

    summary = []
    summary.append("Accepted one-base guard-starbase matcher decode capture")
    summary.append("")
    summary.append("Base-side decode breakpoint: 0814:0403")
    summary.append(base_ev)
    summary.append("")
    summary.append(base_hex)
    summary.append("")
    summary.append("Candidate-side decode breakpoint: 0814:0686")
    summary.append(cand_ev)
    summary.append("")
    summary.append(cand_hex)
    summary.append("")
    (ARTIFACT_DIR / "summary.txt").write_text("\n".join(summary))
    print(ARTIFACT_DIR)


if __name__ == "__main__":
    main()
