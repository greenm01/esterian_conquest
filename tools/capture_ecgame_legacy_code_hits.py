import json
import os
import re
import shutil
import time
from pathlib import Path

from pexpect_argv import spawn_argv


TARGET = Path("/tmp/ecgame-legacy-code-hits")
ARTIFACT_DIR = Path("artifacts/ecgame-startup")


def write_legacy_door_sys(path: Path) -> None:
    text = """COM1:
19200
8
1
19200
Y
Y
Y
Y
Sysop First
Sysop Last
Orlando, FL
1
1
100
9000
1
2
90
90
90
90
90
90
"""
    path.write_bytes(text.replace("\n", "\r\n").encode("ascii"))


def prepare_target() -> None:
    if TARGET.exists():
        shutil.rmtree(TARGET)
    shutil.copytree("fixtures/ecutil-init/v1.5", TARGET)
    shutil.copy2("original/v1.5/ECGAME.EXE", TARGET)
    write_legacy_door_sys(TARGET / "DOOR.SYS")


def read_available(child) -> str:
    text = ""
    while True:
        try:
            text += child.read_nonblocking(size=4096, timeout=0.2)
        except Exception:
            break
    return text


def send(child, cmd_text: str, delay: float = 0.45) -> None:
    child.sendline(cmd_text)
    time.sleep(delay)


def capture_ev(child) -> list[str]:
    send(child, "EV AX BX CX DX SI DI BP SP CS DS ES SS", 0.35)
    text = read_available(child)
    ev_match = re.search(
        r"EV of 'AX BX CX DX SI DI BP SP CS DS ES SS' is:\s*LOG:\s*([0-9a-fA-F ]+)",
        text,
    )
    if not ev_match:
        raise RuntimeError(f"Unable to parse EV output:\n{text}")
    values = ev_match.group(1).split()
    if len(values) != 12:
        raise RuntimeError(f"Unexpected EV field count {len(values)} in:\n{text}")
    return values


def dump_mem(child, segment: str, offset_hex: str, size_hex: str, out_name: str) -> bytes:
    send(child, f"MEMDUMPBIN {segment}:{offset_hex} {size_hex}", 0.8)
    read_available(child)
    data = (TARGET / "MEMDUMP.BIN").read_bytes()
    (ARTIFACT_DIR / out_name).write_bytes(data)
    return data


def capture_hit(child, tag: str, values: list[str], seq: int) -> dict[str, object]:
    ds = values[9].upper()
    ss = values[11].upper()
    bp = int(values[6], 16)
    sp = int(values[7], 16)
    frame_start = max(0, bp - 0x20)
    stack_start = sp

    frame_bytes = dump_mem(child, ss, f"{frame_start:04X}", "60", f"{tag}-{seq:02d}-frame.bin")
    stack_bytes = dump_mem(child, ss, f"{stack_start:04X}", "40", f"{tag}-{seq:02d}-stack.bin")

    return {
        "tag": tag,
        "seq": seq,
        "ax": values[0].upper(),
        "bx": values[1].upper(),
        "cx": values[2].upper(),
        "dx": values[3].upper(),
        "si": values[4].upper(),
        "di": values[5].upper(),
        "bp": values[6].upper(),
        "sp": values[7].upper(),
        "cs": values[8].upper(),
        "ds": ds,
        "ss": ss,
        "frame_start": f"{frame_start:04X}",
        "frame_hex": frame_bytes.hex(),
        "stack_hex": stack_bytes.hex(),
    }


def main() -> None:
    prepare_target()
    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)

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
        time.sleep(2.2)
        send(child, "BPINT 21 3D", 0.7)
        send(child, "RUN", 3.6)
        read_available(child)
        capture_ev(child)  # first live stop

        for cmd_text in ("BPDEL *", "BP 4294:06FC", "BP 4294:076D", "BP 4294:01A3"):
            send(child, cmd_text, 0.25)

        wanted = {"000006FC": "06FC", "0000076D": "076D", "000001A3": "01A3"}
        counts = {"06FC": 0, "076D": 0, "01A3": 0}
        hits: list[dict[str, object]] = []

        while True:
            send(child, "RUN", 2.5)
            text = read_available(child)
            matched = None
            for needle, tag in wanted.items():
                if needle in text:
                    matched = tag
                    break
            if matched is None:
                if "4C00" in text and "4294" in text:
                    break
                continue

            counts[matched] += 1
            values = capture_ev(child)
            hits.append(capture_hit(child, matched, values, counts[matched]))

            if matched == "01A3":
                break
            if counts["06FC"] >= 3 and counts["076D"] >= 1 and counts["01A3"] >= 0:
                # Let the run continue until exit anchor if it is close.
                continue

        out_path = ARTIFACT_DIR / "legacy-door-code-hits.json"
        out_path.write_text(json.dumps(hits, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(hits, indent=2))
    finally:
        try:
            send(child, "EXIT", 0.2)
        except Exception:
            pass
        if child.isalive():
            child.close(force=True)


if __name__ == "__main__":
    main()
