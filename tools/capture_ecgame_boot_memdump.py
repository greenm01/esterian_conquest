#!/usr/bin/env python3
"""Capture ECGAME.EXE memory dump after LZEXE stub decompresses.

Uses DOSBox-X debugger to:
1. Load ECGAME.EXE
2. Set a breakpoint after the stub completes (when CS changes from stub segment)
3. Dump the full conventional memory (640KB)
4. Extract IVT (first 1KB) for use with emu8086.py

Usage:
    python3 capture_ecgame_boot_memdump.py <game_dir> [output_dir]
"""
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def main():
    if len(sys.argv) < 2:
        print("usage: capture_ecgame_boot_memdump.py <game_dir> [output_dir]")
        return 1

    game_dir = Path(sys.argv[1]).resolve()
    out_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(".")

    if not (game_dir / "ECGAME.EXE").exists():
        print(f"ECGAME.EXE not found in {game_dir}")
        return 1

    out_dir.mkdir(parents=True, exist_ok=True)

    # DOSBox-X debugger command to dump memory
    # MEMDUMPBIN saves raw binary memory dumps
    # We'll use a batch file that runs ECGAME briefly then triggers a dump

    # Create a DOSBox-X debugger script
    # The approach: use DOSBox-X's built-in MEMDUMPBIN command
    # which dumps conventional memory to a file

    # Create a batch file that ECGAME will run from
    bat_content = (
        "@echo off\r\n"
        "ECGAME\r\n"
    )
    bat_path = game_dir / "RUNDUMP.BAT"
    bat_path.write_text(bat_content)

    # DOSBox-X config for debugger mode
    # We use -set "startcmd=..." approach or just run with debugger
    # The simplest approach: use MEMDUMPBIN from the DOSBox-X debugger console

    # Actually, the easiest approach is to use DOSBox-X's SAVESTATE or
    # MEMDUMPBIN command. Let's try running DOSBox-X with a debug break.

    # Alternative simpler approach: dump IVT from DOSBox-X at ECGAME load time
    # by using the debugger to break on the CS:IP of ECGAME entry, dump IVT,
    # then let it run, break again after decompression, and dump the full memory.

    # For now, let's do it in two steps:
    # Step 1: Get the IVT by dumping memory right before ECGAME runs
    # Step 2: Dump memory after decompression completes

    # DOSBox-X supports: MEMDUMPBIN seg off len file
    # from the built-in debugger (activated by pressing Alt+Pause or F12+D)

    # Let's create a simpler approach: use DOSBox-X AUTEXEC to trigger
    # a memdump at the right time via the DEBUGBOX tool

    # Actually the simplest: write a config that uses debugger commands
    # DOSBox-X can be started with -debug flag and then scripted

    # Simplest approach: just dump the IVT before running ECGAME
    # and the full memory after by using two sequential MEMDUMPBIN calls

    print("Approach: Using DOSBox-X MEMDUMPBIN to dump IVT")
    print()
    print("Please run DOSBox-X manually with these steps:")
    print(f"  1. dosbox-x -defaultconf -nopromptfolder \\")
    print(f"     -set 'machine=vgaonly' -set 'core=normal' \\")
    print(f"     -set 'cputype=386_prefetch' -set 'cycles=fixed 3000' \\")
    print(f"     -set 'xms=false' -set 'ems=false' -set 'umb=false' \\")
    print(f"     -c 'mount c {game_dir}' -c 'c:'")
    print()
    print("  2. At the C:\\> prompt, press Alt+Pause to open debugger")
    print("  3. In the debugger, type:")
    print(f"       MEMDUMPBIN 0 0 400 {out_dir / 'ivt_before.bin'}")
    print(f"       MEMDUMPBIN 0 0 A0000 {out_dir / 'mem_before.bin'}")
    print("  4. Type 'RUN' or press F5 to close debugger")
    print("  5. At C:\\> type: ECGAME")
    print("  6. Wait for the game menu to appear")
    print("  7. Press Alt+Pause to open debugger again")
    print("  8. In the debugger, type:")
    print(f"       MEMDUMPBIN 0 0 A0000 {out_dir / 'mem_after.bin'}")
    print("  9. Type 'QUIT' to exit")
    print()
    print(f"Output files will be in: {out_dir}")
    print()

    # Alternative: try to automate with xdotool or pexpect
    # For now just provide instructions

    # Actually let's try the automated approach using DOSBox-X's
    # AUTOTYPE feature to type commands automatically

    print("--- Attempting automated capture ---")

    # Create a config that auto-types debugger commands
    # DOSBox-X has AUTOTYPE built in for typing text
    cfg = tempfile.NamedTemporaryFile(mode='w', suffix='.conf', delete=False)
    cfg.write(f"""[dosbox]
machine=vgaonly

[cpu]
core=normal
cputype=386_prefetch
cycles=fixed 3000

[dos]
xms=false
ems=false
umb=false

[autoexec]
mount c {game_dir}
c:
echo Dumping IVT and memory before ECGAME...
rem We can't easily automate debugger from here
rem Instead, let ECGAME run and use SAVESTATE
ECGAME
""")
    cfg_path = cfg.name
    cfg.close()

    # Run DOSBox-X with the debug flag to get debugger access
    # Use SDL_VIDEODRIVER=dummy for headless (won't work for interactive debug)
    # Actually we need the screen for the game. Let's use x11.

    env = os.environ.copy()
    env['SDL_VIDEODRIVER'] = 'x11'
    env['SDL_AUDIODRIVER'] = 'dummy'

    print(f"Config: {cfg_path}")
    print()
    print("Starting DOSBox-X... When the game menu appears:")
    print("  1. Press Alt+Pause to open debugger")
    print(f"  2. Type: MEMDUMPBIN 0 0 A0000 {out_dir / 'mem_after.bin'}")
    print("  3. Press F5 to continue, then close the window")

    try:
        proc = subprocess.Popen(
            ['dosbox-x', '-conf', cfg_path, '-defaultconf', '-nopromptfolder'],
            env=env,
        )
        proc.wait()
    except KeyboardInterrupt:
        proc.kill()
    finally:
        os.unlink(cfg_path)

    # Check if dump was created
    mem_after = out_dir / 'mem_after.bin'
    if mem_after.exists():
        data = mem_after.read_bytes()
        print(f"\nMemory dump: {len(data)} bytes")

        # Extract IVT (first 1024 bytes)
        ivt_path = out_dir / 'ivt.bin'
        ivt_path.write_bytes(data[:0x400])
        print(f"IVT extracted: {ivt_path} ({0x400} bytes)")

        # Search for strings
        for sig in [b'Runtime error', b'Turbo Pascal', b'PLANETS.DAT',
                    b'Esterian', b'CHAIN.TXT']:
            pos = data.find(sig)
            if pos >= 0:
                print(f"Found '{sig.decode()}' at 0x{pos:05x}")
    else:
        print(f"\nNo dump file found at {mem_after}")
        print("You can create it manually using the instructions above.")

    return 0


if __name__ == '__main__':
    raise SystemExit(main())
