#!/usr/bin/env python3
"""Capture decrypted program body from EC executables using DOSBox-X debugger.

Uses BPINT 21 3D to break on the first DOS file-open call, which fires
after the LZEXE stub's stream cipher has decrypted the body but before
the program does significant work. Then dumps the body region via MEMDUMPBIN.
"""
import os
import re
import shutil
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from pexpect_argv import spawn_argv

ROOT = Path(__file__).resolve().parents[1]


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


def capture_ev(child) -> dict[str, int]:
    send(child, "EV CS EIP DS ES SS SP BP AX BX CX DX SI DI", 0.5)
    text = read_available(child)
    ev_match = re.search(
        r"EV of 'CS EIP DS ES SS SP BP AX BX CX DX SI DI' is:\s*LOG:\s*([0-9a-fA-F ]+)",
        text,
    )
    if not ev_match:
        raise RuntimeError(f"Unable to parse EV output:\n{text}")
    parts = [int(p, 16) for p in ev_match.group(1).strip().split()]
    names = ["CS", "EIP", "DS", "ES", "SS", "SP", "BP", "AX", "BX", "CX", "DX", "SI", "DI"]
    return dict(zip(names, parts, strict=True))


def capture_body(exe_name: str, run_cmd: str, fixtures_dir: Path | None = None) -> Path | None:
    """Capture the decrypted body of an EC executable."""
    exe_path = ROOT / "original" / "v1.5" / f"{exe_name}.EXE"
    if not exe_path.exists():
        print(f"  {exe_name}: EXE not found at {exe_path}")
        return None

    disk = exe_path.read_bytes()
    mz = struct.unpack('<16H', disk[:32])
    hdr_size = mz[4] << 4
    cs_rel = mz[0x0B]
    stub_start = hdr_size + (cs_rel << 4)
    body_size = stub_start - hdr_size
    stub = disk[stub_start:]

    # Extract original MZ header from stub+0x1B5
    orig_hdr = struct.unpack('<16H', stub[0x1B5:0x1D5])

    target = Path(f"/tmp/{exe_name.lower()}-dbg-capture")
    if target.exists():
        shutil.rmtree(target)
    target.mkdir(parents=True)

    if fixtures_dir and fixtures_dir.exists():
        for f in fixtures_dir.iterdir():
            if f.is_file():
                shutil.copy2(f, target)
    shutil.copy2(exe_path, target)

    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    env["TERM"] = "dumb"

    cmd = [
        "dosbox-x",
        "-defaultconf", "-nopromptfolder", "-nogui", "-nomenu",
        "-defaultdir", str(target),
        "-set", "dosv=off",
        "-set", "machine=vgaonly",
        "-set", "core=normal",
        "-set", "cputype=386_prefetch",
        "-set", "cycles=fixed 3000",
        "-set", "xms=false",
        "-set", "ems=false",
        "-set", "umb=false",
        "-set", "output=surface",
        "-c", f"mount c {target}",
        "-c", "c:",
        "-c", f"DEBUGBOX {run_cmd}",
    ]

    print(f"  Launching DOSBox-X debugger for {exe_name}...")
    child = spawn_argv(cmd, env=env, timeout=30)

    try:
        time.sleep(3)  # wait for DEBUGBOX to initialize
        read_available(child)

        # Set breakpoint on first DOS file open (after stub decrypts body)
        send(child, "BPINT 21 3D", 0.3)
        send(child, "RUN", 5.0)

        text = read_available(child)

        # Try to get registers to find load_seg
        try:
            regs = capture_ev(child)
            print(f"  Registers: CS=0x{regs['CS']:04X} DS=0x{regs['DS']:04X} "
                  f"ES=0x{regs['ES']:04X} SS=0x{regs['SS']:04X}")
        except RuntimeError:
            print(f"  Could not parse registers, trying DOS MCBS...")
            regs = None

        # Get MCBs to find the program's PSP and load segment
        send(child, "DOS MCBS", 0.8)
        mcbs_text = read_available(child)

        # Look for the program name in MCBs to find the PSP
        psp = None
        for line in mcbs_text.split('\n'):
            if exe_name.upper() in line.upper():
                m = re.search(r'PSP\s+([0-9A-Fa-f]+)', line)
                if m:
                    psp = int(m.group(1), 16)
                    break
        if psp is None and exe_name.upper() in mcbs_text.upper():
            # Try alternate MCB format: "seg owner ... ECMAINT"
            for line in mcbs_text.split('\n'):
                if exe_name.upper() in line.upper():
                    m = re.search(r'([0-9A-Fa-f]{4})', line)
                    if m:
                        psp = int(m.group(1), 16)
                        break

        if psp is None:
            # Fallback: assume standard layout
            psp = 0x0814
            print(f"  Could not find PSP in MCBs, using default 0x{psp:04X}")
        else:
            print(f"  Found PSP: 0x{psp:04X}")

        load_seg = psp + 0x10
        body_start_seg = load_seg
        print(f"  load_seg=0x{load_seg:04X}, body at {body_start_seg:04X}:0000, "
              f"size=0x{body_size:X} ({body_size} bytes)")

        # Dump the body region
        send(child, f"MEMDUMPBIN {body_start_seg:04X}:0000 {body_size:X}", 1.0)
        read_available(child)

        memdump_path = target / "MEMDUMP.BIN"
        if memdump_path.exists():
            body_data = memdump_path.read_bytes()
            if len(body_data) >= body_size:
                body_data = body_data[:body_size]
                print(f"  Got {len(body_data)} bytes")

                # Verify it's decrypted by checking against disk
                disk_body = disk[hdr_size:stub_start]
                diffs = sum(1 for a, b in zip(body_data, disk_body) if a != b)
                print(f"  Bytes differing from disk: {diffs}/{body_size} ({diffs*100//body_size}%)")

                # Build clean EXE
                new_hdr = struct.pack('<16H',
                    0x5A4D, orig_hdr[1], orig_hdr[2], 0, 2,
                    orig_hdr[5], orig_hdr[6], orig_hdr[7], orig_hdr[8],
                    0, orig_hdr[0xA], orig_hdr[0xB], 0x1C, 0, 0, 0
                )
                expected = (orig_hdr[2] - 1) * 512 + orig_hdr[1] if orig_hdr[1] else orig_hdr[2] * 512
                result = new_hdr + body_data
                if len(result) < expected:
                    result += b'\x00' * (expected - len(result))
                elif len(result) > expected:
                    result = result[:expected]

                out_path = ROOT / "tools" / "unlzexe" / f"{exe_name}_CLEAN.EXE"
                out_path.write_bytes(result)
                print(f"  Wrote {out_path}: {len(result)} bytes")

                # Check for known strings
                for sig in [b'Runtime error', b'Esterian', b'PLANETS.DAT',
                            b'Insufficient', b'Borland']:
                    idx = body_data.find(sig)
                    if idx >= 0:
                        print(f"  Found '{sig.decode()}' at body+0x{idx:x}")

                return out_path
            else:
                print(f"  MEMDUMP.BIN too small: {len(body_data)} bytes")
        else:
            print(f"  No MEMDUMP.BIN created")

    finally:
        try:
            child.sendcontrol("c")
            time.sleep(0.5)
            child.sendline("y")
            time.sleep(0.5)
        except Exception:
            pass
        child.close(force=True)

    return None


def main():
    fixtures = ROOT / "fixtures" / "ecmaint-econ-pre" / "v1.5"

    print("=== ECMAINT ===")
    capture_body("ECMAINT", "ECMAINT /R", fixtures)

    print("\n=== ECUTIL ===")
    capture_body("ECUTIL", "ECUTIL", fixtures)

    print("\n=== ECGAME ===")
    capture_body("ECGAME", "ECGAME", fixtures)


if __name__ == "__main__":
    main()
