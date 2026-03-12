import os
import re
import shutil
import time
from pathlib import Path

from ecgame_dropfiles import write_chain_txt
from pexpect_argv import spawn_argv


TARGET = Path("/tmp/ecgame-first-open-ev")


def prepare_target() -> None:
    if TARGET.exists():
        shutil.rmtree(TARGET)
    shutil.copytree("fixtures/ecutil-init/v1.5", TARGET)
    shutil.copy2("original/v1.5/ECGAME.EXE", TARGET)
    write_chain_txt(TARGET / "CHAIN.TXT")


def main() -> None:
    prepare_target()

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
        child.sendline("BPINT 21 3D")
        time.sleep(1)
        child.sendline("RUN")
        time.sleep(10)
        text = ""
        while True:
            try:
                text += child.read_nonblocking(size=4096, timeout=0.2)
            except Exception:
                break

        child.sendline("EV AX BX CX DX SI DI BP SP DS ES SS")
        time.sleep(1)
        while True:
            try:
                text += child.read_nonblocking(size=4096, timeout=0.2)
            except Exception:
                break

        (TARGET / "ev.txt").write_text(text, encoding="utf-8", errors="replace")

        ev_match = re.search(
            r"EV of 'AX BX CX DX SI DI BP SP DS ES SS' is:\s*LOG:\s*([0-9a-fA-F ]+)",
            text,
        )
        if not ev_match:
            raise RuntimeError(f"Unable to parse EV output:\n{text}")
        values = ev_match.group(1).split()
        if len(values) != 11:
            raise RuntimeError(f"Unexpected EV field count {len(values)} in:\n{text}")

        offset = values[3].upper()
        segment = values[8].upper()
        (TARGET / "open_ptr.txt").write_text(
            f"DS={segment}\nDX={offset}\n", encoding="ascii"
        )

        child.sendline(f"MEMDUMPBIN {segment}:{offset} 100")
        time.sleep(2)
        child.sendline("EXIT")
        child.close()
        print(f"Captured first open path pointer {segment}:{offset}")
    finally:
        if child.isalive():
            child.close(force=True)


if __name__ == "__main__":
    main()
