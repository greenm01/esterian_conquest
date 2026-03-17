#!/usr/bin/env python3
"""Capture the ECMAINT startup-to-step-4 driver skeleton.

Sets breakpoints at earlier startup/token seams plus the current best step-4
producer seams, then records the order they fire during a real ECMAINT /R run.
This is intended to recover the path *before* the already-bounded 861d late
tail, not to re-map the late tail itself.

Address translation (PSP base 0814):
  Ghidra 0000:xxxx -> DOSBox 0814:xxxx
  Ghidra 1000:xxxx -> DOSBox 1814:xxxx
  Ghidra 2000:xxxx -> DOSBox 2814:xxxx
  Ghidra 3000:xxxx -> DOSBox 3814:xxxx
"""

from __future__ import annotations

import os
import re
import shutil
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path

from pexpect_argv import spawn_argv

ROOT = Path(__file__).resolve().parents[1]
ARTIFACT_DIR = ROOT / "artifacts" / "ecmaint-sim-driver-trace"

# Breakpoint table: (label, DOSBox seg:off).
# DOSBox may renormalize CS:EIP, so hit identification must compare linear
# addresses rather than raw segments or offsets.
BREAKPOINTS: list[tuple[str, str, int]] = [
    # Startup / recovery seams
    ("summary-workspace-init-9e1e", "2814:9e1e", 0),
    ("move-token-delete-9cb0", "2814:9cb0", 0),
    ("restore-workspace-refresh-731f", "2814:731f", 0),
    ("validation-entry-6d9b", "2814:6d9b", 0),
    ("validate-primary-5ee4", "2814:5ee4", 0),
    # Current step-4 producer seams
    ("durable-kind1-producer-00e8", "1814:00e8", 0x00e8),
    ("durable-kind2-producer-024d", "1814:024d", 0x024d),
    # Durable pool writers inside those producers
    ("kind1-writer-dddb", "1814:dddb", 0xdddb),
    ("kind2-writer-e31b", "1814:e31b", 0xe31b),
    # Already-bounded late boundary, kept only as an upper fence
    ("late-tail-entry-861d", "2814:861d", 0),
    # Weekly emission loop
    ("weekly-loop-entry-127a", "0814:127a", 0x127a),
]

# How many RUN iterations before giving up
MAX_ITERATIONS = 200


@dataclass
class BreakpointHit:
    iteration: int
    label: str
    cs: int
    eip: int
    ds: int
    es: int
    ss: int
    sp: int
    bp: int
    ax: int
    bx: int
    cx: int
    dx: int
    si: int
    di: int
    raw_ev: str
    stack_words: str = ""


def read_available(child, timeout: float = 0.3) -> str:
    text = ""
    while True:
        try:
            text += child.read_nonblocking(size=4096, timeout=timeout)
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


def linear_addr(seg: int, off: int) -> int:
    return ((seg & 0xFFFF) << 4) + (off & 0xFFFF)


def capture_ev(child) -> tuple[str, dict[str, int]]:
    send(child, "EV CS EIP DS ES SS SP BP AX BX CX DX SI DI", 0.5)
    text = read_available(child)
    ev_match = re.search(
        r"EV of 'CS EIP DS ES SS SP BP AX BX CX DX SI DI' is:\s*LOG:\s*([0-9a-fA-F ]+)",
        text,
    )
    if not ev_match:
        raise RuntimeError(f"Unable to parse EV output:\n{text!r}")
    ev_block = ev_match.group(1).strip()
    return ev_block, parse_ev_registers(ev_block)


def capture_stack_words(child, regs: dict[str, int], count: int = 8) -> str:
    """Dump a few words from SS:SP to see return addresses."""
    seg = regs["SS"]
    off = regs["SP"]
    length = count * 2  # 16-bit words
    send(child, f"D {seg:04X}:{off:04X} {length}", 0.5)
    text = read_available(child)
    return text.strip()


def identify_hit(regs: dict[str, int]) -> str | None:
    actual = linear_addr(regs["CS"], regs["EIP"])
    for label, addr, expected_eip in BREAKPOINTS:
        seg_text, off_text = addr.split(":")
        expected = linear_addr(int(seg_text, 16), int(off_text, 16))
        if actual == expected:
            return label
        if expected_eip and regs["EIP"] == expected_eip:
            return label
    return None


def prepare_scenario(fixture_src: Path, target: Path) -> None:
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(fixture_src, target)
    # Ensure ECMAINT.EXE is present
    engine = target / "ECMAINT.EXE"
    if not engine.exists():
        fallback = ROOT / "original" / "v1.5" / "ECMAINT.EXE"
        if fallback.exists():
            shutil.copy2(fallback, engine)


def main() -> int:
    scenario = sys.argv[1] if len(sys.argv) > 1 else "fleet-order"

    # Resolve scenario fixture
    from ecmaint_oracle import KNOWN_SCENARIOS
    if scenario not in KNOWN_SCENARIOS:
        print(f"Unknown scenario: {scenario}")
        print(f"Known: {', '.join(sorted(KNOWN_SCENARIOS))}")
        return 1

    fixture_src = KNOWN_SCENARIOS[scenario]["pre"]
    target = Path(f"/tmp/ecmaint-sim-driver-trace-{scenario}")

    print(f"Preparing scenario: {scenario}")
    print(f"  fixture: {fixture_src}")
    print(f"  target: {target}")

    prepare_scenario(fixture_src, target)

    # First, do a sanity run to make sure ECMAINT works
    print("Running sanity check...")
    sanity_target = Path(f"/tmp/ecmaint-sim-driver-sanity-{scenario}")
    prepare_scenario(fixture_src, sanity_target)
    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    import subprocess
    result = subprocess.run(
        [
            "dosbox-x", "-defaultconf", "-nopromptfolder", "-nogui", "-nomenu",
            "-defaultdir", str(sanity_target),
            "-set", "dosv=off", "-set", "machine=vgaonly", "-set", "core=normal",
            "-set", "cputype=386_prefetch", "-set", "cycles=fixed 3000",
            "-set", "xms=false", "-set", "ems=false", "-set", "umb=false",
            "-set", "output=surface",
            "-c", f"mount c {sanity_target}", "-c", "c:",
            "-c", "ECMAINT /R", "-c", "exit",
        ],
        env=env, capture_output=True, text=True,
    )
    errors_file = sanity_target / "ERRORS.TXT"
    if errors_file.exists():
        error_text = errors_file.read_text(errors="ignore").strip()
        if error_text:
            print(f"  WARNING: ERRORS.TXT: {error_text[:200]}")
    print(f"  Sanity exit code: {result.returncode}")

    # Re-prepare clean target for debug run
    prepare_scenario(fixture_src, target)

    # Launch DOSBox-X in debugger mode
    print("Launching DOSBox-X debugger...")
    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    env["TERM"] = "dumb"

    cmd = [
        "dosbox-x", "-defaultconf", "-nopromptfolder", "-nogui", "-nomenu",
        "-defaultdir", str(target),
        "-set", "dosv=off", "-set", "machine=vgaonly", "-set", "core=normal",
        "-set", "cputype=386_prefetch", "-set", "cycles=fixed 3000",
        "-set", "xms=false", "-set", "ems=false", "-set", "umb=false",
        "-set", "output=surface",
        "-c", f"mount c {target}", "-c", "c:",
        "-c", "DEBUGBOX ECMAINT /R",
    ]

    child = spawn_argv(cmd, env=env, timeout=60, encoding="utf-8")
    transcript: list[str] = []
    hits: list[BreakpointHit] = []

    try:
        # Wait for debugger to initialize
        time.sleep(3)
        transcript.append(read_available(child, 1.0))

        # Arm the real breakpoints immediately. The earlier approach of first
        # stopping on INT 21/3D was too late: by that point ECMAINT had already
        # passed the startup/driver seams we care about.
        send(child, "BPDEL *", 0.5)
        for label, addr, _eip in BREAKPOINTS:
            send(child, f"BP {addr}", 0.3)
            print(f"  BP set: {addr} ({label})")

        # Also break on INT 21/4C (program exit)
        send(child, "BPINT 21 4C", 0.3)

        print(f"\nRunning with {len(BREAKPOINTS)} breakpoints (max {MAX_ITERATIONS} iterations)...")
        print()

        seen_late_tail = False
        seen_early_driver = False
        iteration = 0

        while iteration < MAX_ITERATIONS:
            iteration += 1
            send(child, "RUN", 2.0)
            transcript.append(read_available(child, 0.5))

            try:
                ev_block, regs = capture_ev(child)
            except RuntimeError as e:
                print(f"  [{iteration:3d}] EV parse failed: {e}")
                transcript.append(f"EV PARSE FAILED at iteration {iteration}\n")
                break

            # Check for program exit (INT 21/4C: AH=4C)
            if regs["AX"] >> 8 == 0x4C:
                print(f"  [{iteration:3d}] Program exit (INT 21/4C)")
                break

            label = identify_hit(regs)
            if label is None:
                # Unknown stop — record raw CS:EIP
                label = f"UNKNOWN-{regs['CS']:04X}:{regs['EIP']:04X}"

            # Capture a few stack words for return address context
            stack_text = ""
            try:
                stack_text = capture_stack_words(child, regs, 8)
            except Exception:
                pass

            hit = BreakpointHit(
                iteration=iteration,
                label=label,
                cs=regs["CS"], eip=regs["EIP"],
                ds=regs["DS"], es=regs["ES"],
                ss=regs["SS"], sp=regs["SP"], bp=regs["BP"],
                ax=regs["AX"], bx=regs["BX"], cx=regs["CX"],
                dx=regs["DX"], si=regs["SI"], di=regs["DI"],
                raw_ev=ev_block,
                stack_words=stack_text,
            )
            hits.append(hit)

            # Short summary line
            print(f"  [{iteration:3d}] {label}  CS:EIP={regs['CS']:04X}:{regs['EIP']:04X}  AX={regs['AX']:04X} BX={regs['BX']:04X} CX={regs['CX']:04X} DX={regs['DX']:04X}")

            if label in {
                "summary-workspace-init-9e1e",
                "move-token-delete-9cb0",
                "restore-workspace-refresh-731f",
                "validation-entry-6d9b",
                "validate-primary-5ee4",
                "durable-kind1-producer-00e8",
                "durable-kind2-producer-024d",
            }:
                seen_early_driver = True

            if label == "late-tail-entry-861d":
                seen_late_tail = True
                if seen_early_driver:
                    print("  (Reached late tail after earlier-driver hits)")

            if label == "weekly-loop-entry-127a" and seen_late_tail:
                # We're now in the late emission loop — we can stop after
                # seeing a few iterations to confirm the pattern
                if sum(1 for h in hits if h.label == "weekly-loop-entry-127a") >= 3:
                    print("  (Stopping: seen 3 weekly loop entries after late tail)")
                    break

        # Clean shutdown
        print(f"\nCapture complete: {len(hits)} breakpoint hits in {iteration} iterations")
        child.sendcontrol("c")
        try:
            child.expect(r"y/n:", timeout=5)
            child.sendline("y")
            child.expect_exact("Killed", timeout=5)
            transcript.append(child.before if child.before else "")
        except Exception:
            pass

    finally:
        child.close(force=True)

    # Write artifacts
    if ARTIFACT_DIR.exists():
        shutil.rmtree(ARTIFACT_DIR)
    ARTIFACT_DIR.mkdir(parents=True)

    # Hit sequence summary
    lines = []
    lines.append(f"ECMAINT Simulation Driver Trace — scenario={scenario}")
    lines.append(f"Total breakpoint hits: {len(hits)}")
    lines.append(f"Iterations: {iteration}")
    lines.append("")
    lines.append("=" * 100)
    lines.append("PHASE SEQUENCE (order of breakpoint hits):")
    lines.append("=" * 100)
    lines.append("")

    for hit in hits:
        lines.append(
            f"[{hit.iteration:3d}] {hit.label:<40s}  "
            f"CS:EIP={hit.cs:04X}:{hit.eip:04X}  "
            f"AX={hit.ax:04X} BX={hit.bx:04X} CX={hit.cx:04X} DX={hit.dx:04X} "
            f"SI={hit.si:04X} DI={hit.di:04X}"
        )

    lines.append("")
    lines.append("=" * 100)
    lines.append("UNIQUE PHASE ORDER (first occurrence only):")
    lines.append("=" * 100)
    lines.append("")

    seen = set()
    for hit in hits:
        if hit.label not in seen:
            seen.add(hit.label)
            lines.append(f"  {len(seen):2d}. {hit.label}  (first at iteration {hit.iteration})")

    lines.append("")
    lines.append("=" * 100)
    lines.append("HIT COUNTS:")
    lines.append("=" * 100)
    lines.append("")

    from collections import Counter
    counts = Counter(h.label for h in hits)
    for label, count in counts.most_common():
        lines.append(f"  {label:<40s}  {count:3d} hits")

    summary_text = "\n".join(lines) + "\n"
    (ARTIFACT_DIR / "phase_sequence.txt").write_text(summary_text)
    print(summary_text)

    # Detailed hit log with stack words
    detail_lines = []
    for hit in hits:
        detail_lines.append(f"--- Hit {hit.iteration}: {hit.label} ---")
        detail_lines.append(f"  EV: {hit.raw_ev}")
        detail_lines.append(f"  CS:EIP = {hit.cs:04X}:{hit.eip:04X}")
        detail_lines.append(f"  SS:SP  = {hit.ss:04X}:{hit.sp:04X}  BP={hit.bp:04X}")
        detail_lines.append(f"  AX={hit.ax:04X} BX={hit.bx:04X} CX={hit.cx:04X} DX={hit.dx:04X}")
        detail_lines.append(f"  SI={hit.si:04X} DI={hit.di:04X}")
        if hit.stack_words:
            detail_lines.append(f"  Stack dump:\n{hit.stack_words}")
        detail_lines.append("")
    (ARTIFACT_DIR / "hit_details.txt").write_text("\n".join(detail_lines))

    # Raw transcript
    (ARTIFACT_DIR / "session.log").write_text("".join(transcript))

    print(f"\nArtifacts written to: {ARTIFACT_DIR}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
