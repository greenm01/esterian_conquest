#!/usr/bin/env python3
"""Capture Borland Pascal RandSeed at the first FLEETS.DAT write event.

The first FLEETS.DAT write marks the start of the fleet-processing loop.
At that point, RandSeed (DS:0x03A6, 32-bit LE) reflects the accumulated
PRNG state after validation/loading. Reading it gives us the seed that
was used to generate the fleet visit order.

Strategy:
  1. Stage through first-file-open → 96c4 bridge (standard two-stage load)
  2. Arm INT 21h/40h (file write) breakpoint
  3. Run until a write to FLEETS.DAT (file handle check)
  4. Read DS:0x03A6 (4 bytes) via DOSBox D command
  5. Also capture the first few fleet record write offsets to verify visit order

Usage:
    python3 tools/capture_randseed.py [scenario]
    python3 tools/capture_randseed.py all
"""

from __future__ import annotations

import os
import re
import shutil
import struct
import sys
import time
from pathlib import Path

from pexpect_argv import spawn_argv

ROOT = Path(__file__).resolve().parents[1]

BRIDGE_BP = "2814:96c4"
FLEET_RECORD_SIZE = 54


def read_available(child, timeout: float = 0.3) -> str:
    text = ""
    while True:
        try:
            text += child.read_nonblocking(size=4096, timeout=timeout)
        except Exception:
            break
    return text


def send(child, cmd: str, delay: float = 0.5) -> str:
    child.sendline(cmd)
    time.sleep(delay)
    return read_available(child)


def parse_ev(child) -> dict[str, int]:
    text = send(child, "EV CS EIP DS ES SS SP BP AX BX CX DX SI DI", 0.5)
    m = re.search(
        r"EV of 'CS EIP DS ES SS SP BP AX BX CX DX SI DI' is:\s*LOG:\s*([0-9a-fA-F ]+)",
        text,
    )
    if not m:
        raise RuntimeError(f"EV parse failed:\n{text!r}")
    parts = [int(x, 16) for x in m.group(1).split()]
    names = ["CS", "EIP", "DS", "ES", "SS", "SP", "BP", "AX", "BX", "CX", "DX", "SI", "DI"]
    return dict(zip(names, parts, strict=True))


def read_memory(child, seg: int, off: int, length: int) -> bytes:
    """Read bytes from DOSBox memory via EV of memory expressions."""
    # DOSBox DEBUGBOX D command output is unreliable to parse.
    # Instead, use EV to read individual words.
    result = bytearray()
    for i in range(0, length, 2):
        addr = off + i
        text = send(child, f"EV word [DS:{addr:04X}]", 0.5)
        # Parse: "EV of 'word [DS:XXXX]' is: LOG:  YYYY"
        m = re.search(r"is:\s*LOG:\s*([0-9a-fA-F]+)", text)
        if m:
            val = int(m.group(1), 16)
            result.append(val & 0xFF)
            result.append((val >> 8) & 0xFF)
        else:
            # Try alternate: just look for a hex number after LOG:
            m2 = re.search(r"LOG:\s+([0-9a-fA-F]{1,8})", text)
            if m2:
                val = int(m2.group(1), 16)
                result.append(val & 0xFF)
                result.append((val >> 8) & 0xFF)
            else:
                print(f"    WARNING: could not parse EV output for [{seg:04X}:{addr:04X}]: {text[:100]!r}")
                result.extend(b'\x00\x00')
    return bytes(result[:length])


def prepare(fixture_src: Path, target: Path) -> None:
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(fixture_src, target)
    engine = target / "ECMAINT.EXE"
    if not engine.exists():
        shutil.copy2(ROOT / "original" / "v1.5" / "ECMAINT.EXE", engine)


def capture_randseed_for_scenario(scenario: str, fixture_src: Path) -> dict | None:
    target = Path(f"/tmp/ecmaint-randseed-{scenario}")
    prepare(fixture_src, target)

    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    env["TERM"] = "dumb"

    cmd = [
        "dosbox-x", "-defaultconf", "-nopromptfolder", "-nogui", "-nomenu",
        "-defaultdir", str(target),
        "-set", "dosv=off", "-set", "machine=vgaonly", "-set", "core=normal",
        "-set", "cputype=386_prefetch", "-set", "cycles=fixed 50000",
        "-set", "xms=false", "-set", "ems=false", "-set", "umb=false",
        "-set", "output=surface",
        "-c", f"mount c {target}", "-c", "c:",
        "-c", "DEBUGBOX ECMAINT /R",
    ]

    child = spawn_argv(cmd, env=env, timeout=60, encoding="utf-8")

    try:
        time.sleep(3)
        read_available(child, 1.0)

        # Stage 1: first file-open
        send(child, "BPDEL *")
        send(child, "BPINT 21 3D", 0.3)
        send(child, "BPINT 21 4C", 0.3)
        send(child, "RUN", 3.0)
        read_available(child, 0.8)

        regs = parse_ev(child)
        if regs["AX"] >> 8 == 0x4C:
            print(f"  {scenario}: exited before file-open")
            return None

        # Stage 2: bridge to unpacked image
        send(child, "BPDEL *")
        send(child, f"BP {BRIDGE_BP}", 0.3)
        send(child, "BPINT 21 4C", 0.3)
        send(child, "RUN", 3.0)
        read_available(child, 0.8)

        regs = parse_ev(child)
        if regs["AX"] >> 8 == 0x4C:
            print(f"  {scenario}: exited before bridge")
            return None

        ds_value = regs["DS"]
        print(f"  Bridge hit: DS={ds_value:04X}")

        # Read initial RandSeed
        seed_bytes = read_memory(child, ds_value, 0x03A6, 4)
        if len(seed_bytes) >= 4:
            initial_seed = struct.unpack_from("<I", seed_bytes, 0)[0]
            print(f"  Initial RandSeed (at bridge): 0x{initial_seed:08X}")
        else:
            print(f"  WARNING: could not read initial RandSeed (got {len(seed_bytes)} bytes)")
            initial_seed = None

        # Stage 3: use code breakpoints at key phases to capture RandSeed.
        # The bridge was at 2814:96c4 (Ghidra 2000:96c4).
        # We know the code segment for Ghidra seg 2000 is 0x2814 in DOSBox.
        # But DOSBox may renormalize CS:EIP, so also accept EIP-only matches.
        #
        # Key addresses (Ghidra -> DOSBox):
        #   2000:861d -> 2814:861d  (the call to 6d9b, BEFORE validation)
        #   2000:8652 -> 2814:8652  (AFTER validation, start of late tail)
        #
        # 861d is better than 6d9b because:
        #   - 861d is the CALL instruction site (we catch it before 6d9b runs)
        #   - 8652 is right after the JNZ that skips error handling
        #
        # Strategy: BP at 861d (pre-validation), read RandSeed.
        #           BP at 8652 (post-validation/fleet-loop), read RandSeed.
        #           The fleet loop + economy happen inside the 6d9b call.

        send(child, "BPDEL *")
        # Use just one BP first to verify it works
        send(child, "BP 2814:861d", 0.3)
        send(child, "BPINT 21 4C", 0.3)
        print("  BPs set: 2814:861d (pre-validation), INT 21/4C (exit)")

        seeds_at_phases = {}

        # Phase 1: run to 861d (pre-validation)
        print("  Running to 861d...")
        child.sendline("RUN")
        # Wait for breakpoint hit - DOSBox debugger may take a while
        time.sleep(30)
        text = read_available(child, 5.0)
        print(f"    RUN output ({len(text)} chars): {text[:200]!r}")

        regs = parse_ev(child)
        eip = regs["EIP"]
        cs = regs["CS"]
        print(f"    Stopped at CS:EIP={cs:04X}:{eip:04X} AX={regs['AX']:04X}")

        if regs["AX"] >> 8 == 0x4C:
            print(f"  Program exited before 861d")
            return {"scenario": scenario, "ds": ds_value, "initial_seed": initial_seed, "seeds_at_phases": {}}

        # Read RandSeed at this point
        seed_bytes = read_memory(child, ds_value, 0x03A6, 4)
        seed_pre = struct.unpack_from("<I", seed_bytes, 0)[0] if len(seed_bytes) >= 4 else None
        seeds_at_phases["pre-validation-861d"] = seed_pre
        print(f"  RandSeed at 861d (pre-validation): 0x{seed_pre:08X}" if seed_pre else "  RandSeed: UNKNOWN")

        # Phase 2: set BP at 8652 (post-validation, after fleet loop)
        send(child, "BPDEL *")
        send(child, "BP 2814:8652", 0.3)
        send(child, "BPINT 21 4C", 0.3)
        print("  Running to 8652 (post-validation/fleet-loop)...")
        send(child, "RUN", 30.0)
        text = read_available(child, 3.0)

        regs = parse_ev(child)
        eip = regs["EIP"]
        cs = regs["CS"]
        print(f"    Stopped at CS:EIP={cs:04X}:{eip:04X} AX={regs['AX']:04X}")

        if regs["AX"] >> 8 == 0x4C:
            print(f"  Program exited before 8652")
        else:
            seed_bytes = read_memory(child, ds_value, 0x03A6, 4)
            seed_post = struct.unpack_from("<I", seed_bytes, 0)[0] if len(seed_bytes) >= 4 else None
            seeds_at_phases["post-fleet-loop-8652"] = seed_post
            print(f"  RandSeed at 8652 (post-fleet-loop): 0x{seed_post:08X}" if seed_post else "  RandSeed: UNKNOWN")

        return {
            "scenario": scenario,
            "ds": ds_value,
            "initial_seed": initial_seed,
            "seeds_at_phases": seeds_at_phases,
        }

    finally:
        try:
            child.sendcontrol("c")
            time.sleep(0.5)
            child.sendline("y")
            time.sleep(0.5)
        except Exception:
            pass
        child.close(force=True)


def main() -> int:
    from ecmaint_oracle import KNOWN_SCENARIOS

    scenarios = sys.argv[1:] if len(sys.argv) > 1 else ["bombard"]
    if scenarios == ["all"]:
        scenarios = ["bombard", "econ", "fleet-order", "planet-build"]

    results = []
    for scenario in scenarios:
        if scenario not in KNOWN_SCENARIOS:
            print(f"Unknown scenario: {scenario}")
            continue
        print(f"\n{'='*60}")
        print(f"Scenario: {scenario}")
        print(f"{'='*60}")
        result = capture_randseed_for_scenario(scenario, KNOWN_SCENARIOS[scenario]["pre"])
        if result:
            results.append(result)

    # Summary
    print(f"\n{'='*60}")
    print("RANDSEED SUMMARY")
    print(f"{'='*60}")
    for r in results:
        print(f"\n  {r['scenario']}:")
        print(f"    DS = 0x{r['ds']:04X}")
        if r['initial_seed'] is not None:
            print(f"    initial (bridge)    = 0x{r['initial_seed']:08X}")
        for phase, seed in r.get('seeds_at_phases', {}).items():
            if seed is not None:
                print(f"    {phase:22s} = 0x{seed:08X}")

    # Write artifact
    artifact_dir = ROOT / "artifacts" / "ecmaint-randseed"
    artifact_dir.mkdir(parents=True, exist_ok=True)
    lines = ["ECMAINT RandSeed Capture Results", ""]
    for r in results:
        lines.append(f"Scenario: {r['scenario']}")
        lines.append(f"  DS segment: 0x{r['ds']:04X}")
        if r['initial_seed'] is not None:
            lines.append(f"  Initial RandSeed (at bridge): 0x{r['initial_seed']:08X}")
        for phase, seed in r.get('seeds_at_phases', {}).items():
            lines.append(f"  RandSeed at {phase}: 0x{seed:08X}" if seed is not None
                         else f"  RandSeed at {phase}: unknown")
        lines.append("")
    (artifact_dir / "randseed_capture.txt").write_text("\n".join(lines))
    print(f"\nArtifact written to: {artifact_dir / 'randseed_capture.txt'}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
