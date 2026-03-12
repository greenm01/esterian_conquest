import os
import re
import shutil
import time
from pathlib import Path

import pexpect

from ecgame_dropfiles import write_chain_txt
from pexpect_argv import spawn_argv


TARGET = Path("/tmp/ecgame-input-chain")
MEMDUMP_SIZE = 0x97EB0


def prepare_target() -> None:
    if TARGET.exists():
        shutil.rmtree(TARGET)
    shutil.copytree("fixtures/ecutil-init/v1.5", TARGET)
    shutil.copy2("original/v1.5/ECGAME.EXE", TARGET)
    write_chain_txt(TARGET / "CHAIN.TXT")


def get_prompt(child: pexpect.spawn) -> None:
    child.expect([r"I->", r"> _", r"DBG>", r"LOG:"], timeout=10)
    time.sleep(0.1)


def find_ecgame_psp(child: pexpect.spawn) -> str | None:
    child.sendline("DOS MCBS")
    child.expect([r"ECGAME", r"END OF CHAIN"], timeout=5)
    text = child.before + child.after
    try:
        text += child.read_nonblocking(size=1024, timeout=0.2)
    except Exception:
        pass
    (TARGET / "DOS_MCBS.txt").write_text(text, encoding="utf-8", errors="replace")
    match = re.search(r"(?m)^([0-9A-F]{4})\s+[0-9A-F]{4}\s+[0-9A-F]+\s+[0-9A-F]+\s+ECGAME", text)
    get_prompt(child)
    if not match:
        return None
    return match.group(1)


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
        "DEBUGBOX ECGAME.EXE /L",
    ]

    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    env["TERM"] = "dumb"

    child = spawn_argv(cmd, env=env, timeout=20, encoding="cp437")
    try:
        get_prompt(child)
        child.sendline("BPINT 16 00")
        get_prompt(child)
        psp = None
        for _ in range(8):
            child.sendline("RUN")
            time.sleep(2)
            get_prompt(child)
            psp = find_ecgame_psp(child)
            if psp is not None:
                break
        if psp is None:
            raise RuntimeError("ECGAME never appeared in DOS MCBS after repeated RUN cycles")
        print(f"ECGAME PSP = {psp}")
        child.sendline(f"MEMDUMPBIN {psp}:0000 {MEMDUMP_SIZE:x}")
        get_prompt(child)
        child.sendline("EXIT")
        child.close()
        print(f"Captured {TARGET / 'MEMDUMP.BIN'}")
    finally:
        if child.isalive():
            child.close(force=True)


if __name__ == "__main__":
    main()
