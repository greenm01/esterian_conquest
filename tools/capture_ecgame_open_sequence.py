import os
import re
import shutil
import time
from pathlib import Path

from ecgame_dropfiles import write_chain_txt
from pexpect_argv import spawn_argv


TARGET = Path("/tmp/ecgame-open-sequence")
MAX_OPENS = 12
MAX_NON_OPEN_STOPS = 12


def prepare_target() -> None:
    if TARGET.exists():
        shutil.rmtree(TARGET)
    shutil.copytree("fixtures/ecutil-init/v1.5", TARGET)
    shutil.copy2("original/v1.5/ECGAME.EXE", TARGET)
    write_chain_txt(TARGET / "CHAIN.TXT")


def read_available(child) -> str:
    text = ""
    while True:
        try:
            text += child.read_nonblocking(size=4096, timeout=0.2)
        except Exception:
            break
    return text


def parse_ev(text: str) -> tuple[str, str]:
    ev_match = re.search(
        r"EV of 'AX BX CX DX SI DI BP SP DS ES SS' is:\s*LOG:\s*([0-9a-fA-F ]+)",
        text,
    )
    if not ev_match:
        raise RuntimeError(f"Unable to parse EV output:\n{text}")
    values = ev_match.group(1).split()
    if len(values) != 11:
        raise RuntimeError(f"Unexpected EV field count {len(values)} in:\n{text}")
    return values[8].upper(), values[3].upper()


def decode_dump(path: Path) -> str:
    data = path.read_bytes()
    zero = data.find(b"\x00")
    if zero < 0:
        zero = len(data)
    return data[:zero].decode("cp437", errors="replace")


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

        sequence: list[str] = []
        transcript_parts: list[str] = []

        non_open_stops = 0
        while len(sequence) < MAX_OPENS and non_open_stops < MAX_NON_OPEN_STOPS:
            child.sendline("RUN")
            time.sleep(4)
            text = read_available(child)
            transcript_parts.append(text)
            if "3D02" not in text:
                non_open_stops += 1
                continue

            child.sendline("EV AX BX CX DX SI DI BP SP DS ES SS")
            time.sleep(1)
            ev_text = read_available(child)
            transcript_parts.append(ev_text)
            segment, offset = parse_ev(text + ev_text)
            non_open_stops = 0

            child.sendline(f"MEMDUMPBIN {segment}:{offset} 100")
            time.sleep(1)
            read_available(child)
            dump_path = TARGET / "MEMDUMP.BIN"
            if not dump_path.exists():
                break
            name = decode_dump(dump_path)
            sequence.append(name)
            dump_path.unlink()

        (TARGET / "open_sequence.txt").write_text(
            "\n".join(f"{idx+1}: {name}" for idx, name in enumerate(sequence)) + "\n",
            encoding="utf-8",
        )
        (TARGET / "transcript.txt").write_text("".join(transcript_parts), encoding="utf-8", errors="replace")

        child.sendline("EXIT")
        child.close()
        print(f"Captured {len(sequence)} open(s)")
    finally:
        if child.isalive():
            child.close(force=True)


if __name__ == "__main__":
    main()
