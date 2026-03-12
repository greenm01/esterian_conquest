import json
import os
import re
import shutil
import time
from pathlib import Path

from ecgame_dropfiles import write_chain_txt, write_door_sys
from pexpect_argv import spawn_argv


TARGET_ROOT = Path("/tmp/ecgame-dropfile-probe")
ARTIFACT_DIR = Path("artifacts/ecgame-startup")


VARIANTS = ("chain_only", "door_only", "both")


def prepare_variant(name: str) -> Path:
    target = TARGET_ROOT / name
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree("fixtures/ecutil-init/v1.5", target)
    shutil.copy2("original/v1.5/ECGAME.EXE", target)
    if name in ("chain_only", "both"):
        write_chain_txt(target / "CHAIN.TXT")
    if name in ("door_only", "both"):
        write_door_sys(target / "DOOR.SYS", user_first_name="Sysop", user_last_name="HANNIBAL")
    return target


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


def capture_ev(child) -> list[str]:
    send(child, "EV AX BX CX DX SI DI BP SP CS DS ES SS", 0.5)
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


def arm_breakpoints(child, ah_values: tuple[str, ...]) -> None:
    send(child, "BPDEL *", 0.5)
    for ah in ah_values:
        send(child, f"BPINT 21 {ah}", 0.3)


def dump_name(child, target: Path, values: list[str]) -> str:
    ds = values[9].upper()
    dx = values[3].upper()
    send(child, f"MEMDUMPBIN {ds}:{dx} 100", 1.0)
    read_available(child)
    return (target / "MEMDUMP.BIN").read_bytes().split(b"\x00")[0].decode("cp437", errors="replace")


def run_variant(target: Path) -> dict[str, object]:
    cmd = [
        "dosbox-x",
        "-defaultconf",
        "-nopromptfolder",
        "-nogui",
        "-nomenu",
        "-defaultdir",
        str(target),
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
        f"mount c {target}",
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

        send(child, "BPINT 21 3D", 1.0)
        send(child, "RUN", 4.0)
        read_available(child)
        first_open = capture_ev(child)
        first_name = dump_name(child, target, first_open)

        arm_breakpoints(child, ("3F", "3E", "3D", "4C"))
        sequence: list[str] = []
        second_name: str | None = None
        exit_code: str | None = None
        for _ in range(10):
            send(child, "RUN", 4.0)
            read_available(child)
            values = capture_ev(child)
            ax = values[0].upper()
            sequence.append(ax)
            if ax.startswith("3D"):
                second_name = dump_name(child, target, values)
                arm_breakpoints(child, ("3F", "3E", "3D", "4C"))
            if ax.startswith("4C"):
                exit_code = values[1].upper()
                break

        return {
            "first_open": first_name,
            "second_open": second_name,
            "sequence": sequence,
            "exit_code": exit_code,
        }
    finally:
        try:
            send(child, "EXIT", 0.2)
        except Exception:
            pass
        if child.isalive():
            child.close(force=True)


def main() -> None:
    if TARGET_ROOT.exists():
        shutil.rmtree(TARGET_ROOT)
    TARGET_ROOT.mkdir(parents=True)
    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)

    results: dict[str, dict[str, object]] = {}
    for name in VARIANTS:
        target = prepare_variant(name)
        results[name] = run_variant(target)

    artifact_path = ARTIFACT_DIR / "dropfile-probe.json"
    artifact_path.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
