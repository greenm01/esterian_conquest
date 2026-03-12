import os
import shutil
import time
from pathlib import Path

from ecgame_dropfiles import write_chain_txt
from pexpect_argv import spawn_argv


TARGET = Path("/tmp/ecgame-autodetect-break")


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
        child.sendline("BPINT 16 00")
        time.sleep(1)
        child.sendline("RUN")
        time.sleep(8)
        child.sendline("DOS MCBS")
        time.sleep(2)
        text = ""
        while True:
            try:
                text += child.read_nonblocking(size=4096, timeout=0.2)
            except Exception:
                break
        (TARGET / "DOS_MCBS.txt").write_text(text, encoding="utf-8", errors="replace")
        child.sendline("MEMDUMPBIN B800:0000 4000")
        time.sleep(2)
        child.sendline("EXIT")
        child.close()
        print(f"Finished {TARGET}")
    finally:
        if child.isalive():
            child.close(force=True)


if __name__ == "__main__":
    main()
