import os
import re
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ECMAINT = ROOT / "original" / "v1.5" / "ECMAINT.EXE"
GOOD_FIXTURE = ROOT / "fixtures" / "ecmaint-starbase-pre" / "v1.5"
ARTIFACT_DIR = ROOT / "artifacts" / "ecmaint-validation-trace"
WORK_ROOT = Path("/tmp/ecmaint-validation-trace")

BASE_RECORD_HEX = (
    "0100010001000001000000100d80000000000080000000000081000000000000100d01"
)

READ_RE = re.compile(r"DEBUG FILES:Reading (\d+) bytes from ([^ ]+)")
WRITE_RE = re.compile(r"DEBUG FILES:Writing (\d+) bytes to ([^ ]+)")
OPEN_RE = re.compile(r"FILES:(?:file open command \d+ file|Special file open command \d+ file) (.+)")
SEEK_RE = re.compile(r"DEBUG FILES:Seeking to (\d+) bytes from position type \((\d+)\) in ([^ ]+)")

TRACKED = {
    "CONQUEST.DAT",
    "SETUP.DAT",
    "PLAYER.DAT",
    "PLANETS.DAT",
    "FLEETS.DAT",
    "BASES.DAT",
    "IPBM.DAT",
    "DATABASE.DAT",
    "RANKINGS.TXT",
    "ERRORS.TXT",
    "MAIN.TOK",
    "PLAYER.TOK",
    "PLANETS.TOK",
    "FLEETS.TOK",
    "DATABASE.TOK",
    "CONQUEST.TOK",
}

CORE_FILES = {
    "CONQUEST.DAT",
    "SETUP.DAT",
    "PLAYER.DAT",
    "PLANETS.DAT",
    "FLEETS.DAT",
    "BASES.DAT",
    "IPBM.DAT",
    "DATABASE.DAT",
    "RANKINGS.TXT",
    "ERRORS.TXT",
}


@dataclass
class Event:
    kind: str
    name: str
    detail: str


def build_two_base_file() -> bytes:
    base1 = bytearray.fromhex(BASE_RECORD_HEX)
    base2 = bytearray.fromhex(BASE_RECORD_HEX)
    base1[0x08] = 0x02
    base2[0x00] = 0x02
    base2[0x02] = 0x01
    base2[0x04] = 0x02
    base2[0x05] = 0x01
    base2[0x07] = 0x01
    base2[0x0B] = 0x04
    base2[0x0C] = 0x0D
    base2[0x1E] = 0x04
    base2[0x1F] = 0x0D
    return bytes(base1) + bytes(base2)


def reset_dir(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True)


def prepare_good(path: Path) -> None:
    shutil.copytree(GOOD_FIXTURE, path, dirs_exist_ok=True)
    shutil.copy2(ECMAINT, path / "ECMAINT.EXE")


def prepare_bad(path: Path) -> None:
    prepare_good(path)
    player = bytearray((path / "PLAYER.DAT").read_bytes())
    player[0x44:0x48] = bytes([0x02, 0x00, 0x02, 0x00])
    (path / "PLAYER.DAT").write_bytes(player)
    (path / "BASES.DAT").write_bytes(build_two_base_file())


def prepare_bad_with_token(path: Path, token_name: str) -> None:
    prepare_bad(path)
    (path / token_name).write_bytes(b"")


def run_trace(path: Path, log_path: Path) -> None:
    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    cmd = [
        "dosbox-x",
        "-defaultconf",
        "-nopromptfolder",
        "-nogui",
        "-nomenu",
        "-defaultdir",
        str(path),
        "-debug",
        "-log-int21",
        "-log-fileio",
        "-time-limit",
        "12",
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
        f"mount c {path}",
        "-c",
        "c:",
        "-c",
        "ECMAINT /R",
        "-c",
        "exit",
    ]
    with log_path.open("w") as handle:
        subprocess.run(cmd, stdout=handle, stderr=subprocess.STDOUT, env=env, check=False)


def normalize_name(name: str) -> str:
    name = name.strip().strip('"')
    if "\\" in name:
        name = name.rsplit("\\", 1)[-1]
    return name.upper()


def parse_trace(log_path: Path) -> list[Event]:
    events: list[Event] = []
    armed = False
    for line in log_path.read_text(errors="ignore").splitlines():
        if "Execute ECMAINT.EXE" in line:
            armed = True
        if not armed:
            continue

        match = OPEN_RE.search(line)
        if match:
            name = normalize_name(match.group(1))
            if name in TRACKED:
                events.append(Event("open", name, ""))
            continue

        match = READ_RE.search(line)
        if match:
            name = normalize_name(match.group(2))
            if name in TRACKED:
                events.append(Event("read", name, match.group(1)))
            continue

        match = WRITE_RE.search(line)
        if match:
            name = normalize_name(match.group(2))
            if name in TRACKED:
                events.append(Event("write", name, match.group(1)))
            continue

        match = SEEK_RE.search(line)
        if match:
            name = normalize_name(match.group(3))
            if name in TRACKED:
                detail = f"offset={match.group(1)} whence={match.group(2)}"
                events.append(Event("seek", name, detail))
    return events


def write_summary(name: str, events: list[Event], output_path: Path, error_text: str | None) -> None:
    with output_path.open("w") as out:
        out.write(f"Scenario: {name}\n")
        if error_text:
            out.write(f"ERRORS.TXT first line: {error_text}\n")
        else:
            out.write("ERRORS.TXT first line: <none>\n")
        out.write("\n")
        for event in events:
            suffix = f" {event.detail}" if event.detail else ""
            out.write(f"{event.kind:5} {event.name}{suffix}\n")


def compare_events(good: list[Event], bad: list[Event]) -> str:
    lines: list[str] = []
    lines.append("Early validation comparison")
    lines.append("")

    def slice_until_decision(events: list[Event]) -> list[Event]:
        trimmed: list[Event] = []
        for event in events:
            trimmed.append(event)
            if event.kind == "write" and event.name == "ERRORS.TXT":
                break
            if event.kind == "write" and event.name in {
                "PLAYER.DAT",
                "PLANETS.DAT",
                "FLEETS.DAT",
                "BASES.DAT",
                "CONQUEST.DAT",
                "DATABASE.DAT",
                "RANKINGS.TXT",
            }:
                break
        return trimmed

    good_early = slice_until_decision(good)
    bad_early = slice_until_decision(bad)

    limit = min(len(good_early), len(bad_early))
    divergence = limit
    for idx in range(limit):
        if good_early[idx] != bad_early[idx]:
            divergence = idx
            break

    def compress_prefix(events: list[Event]) -> list[str]:
        result: list[str] = []
        idx = 0
        while idx < len(events):
            event = events[idx]
            if event.kind == "read":
                count = 0
                detail = event.detail
                name = event.name
                while idx < len(events) and events[idx].kind == "read" and events[idx].name == name and events[idx].detail == detail:
                    count += 1
                    idx += 1
                if count == 1:
                    result.append(f"read {name} {detail}")
                else:
                    result.append(f"read {name} {detail} x{count}")
                continue
            suffix = f" {event.detail}" if event.detail else ""
            result.append(f"{event.kind} {event.name}{suffix}")
            idx += 1
        return result

    shared_prefix = compress_prefix(good_early[:divergence])
    lines.append("Shared prefix before divergence:")
    for item in shared_prefix:
        lines.append(f"- {item}")

    lines.append("")
    lines.append("First divergence:")
    if divergence == limit and len(good_early) == len(bad_early):
        lines.append("- no divergence inside the trimmed early sequences")
    else:
        if divergence < len(good_early):
            event = good_early[divergence]
            suffix = f" {event.detail}" if event.detail else ""
            lines.append(f"- good: {event.kind} {event.name}{suffix}")
        if divergence < len(bad_early):
            event = bad_early[divergence]
            suffix = f" {event.detail}" if event.detail else ""
            lines.append(f"- bad: {event.kind} {event.name}{suffix}")

    lines.append("")
    lines.append("Immediate follow-on:")
    for offset in range(1, 6):
        idx = divergence + offset
        if idx < len(good_early):
            event = good_early[idx]
            suffix = f" {event.detail}" if event.detail else ""
            lines.append(f"- good+{offset}: {event.kind} {event.name}{suffix}")
        if idx < len(bad_early):
            event = bad_early[idx]
            suffix = f" {event.detail}" if event.detail else ""
            lines.append(f"- bad+{offset}: {event.kind} {event.name}{suffix}")

    lines.append("")
    lines.append("Practical difference:")
    bad_error_write = next((event for event in bad_early if event.kind == "write" and event.name == "ERRORS.TXT"), None)
    good_game_write = next(
        (
            event
            for event in good_early
            if event.kind == "write"
            and event.name in {"PLAYER.DAT", "PLANETS.DAT", "FLEETS.DAT", "BASES.DAT", "CONQUEST.DAT", "DATABASE.DAT", "RANKINGS.TXT"}
        ),
        None,
    )
    if bad_error_write and good_game_write:
        lines.append(
            f"- after the shared read/seek sweep, the failing case writes {bad_error_write.name} first while the good case reaches normal maintenance output via {good_game_write.name}"
        )
    return "\n".join(lines) + "\n"


def compare_core_validation(passing: list[Event], failing: list[Event]) -> str:
    lines: list[str] = []
    lines.append("Core validation comparison")
    lines.append("")

    passing_core = [event for event in passing if event.name in CORE_FILES]
    failing_core = [event for event in failing if event.name in CORE_FILES]

    def slice_until_decision(events: list[Event]) -> list[Event]:
        trimmed: list[Event] = []
        for event in events:
            trimmed.append(event)
            if event.kind == "write" and event.name == "ERRORS.TXT":
                break
            if event.kind == "open" and event.name == "IPBM.DAT":
                break
        return trimmed

    passing_early = slice_until_decision(passing_core)
    failing_early = slice_until_decision(failing_core)

    limit = min(len(passing_early), len(failing_early))
    divergence = limit
    for idx in range(limit):
        if passing_early[idx] != failing_early[idx]:
            divergence = idx
            break

    lines.append("First divergence:")
    if divergence == limit and len(passing_early) == len(failing_early):
        lines.append("- no divergence inside the trimmed core sequences")
    else:
        if divergence < len(passing_early):
            event = passing_early[divergence]
            suffix = f" {event.detail}" if event.detail else ""
            lines.append(f"- passing: {event.kind} {event.name}{suffix}")
        if divergence < len(failing_early):
            event = failing_early[divergence]
            suffix = f" {event.detail}" if event.detail else ""
            lines.append(f"- failing: {event.kind} {event.name}{suffix}")

    lines.append("")
    lines.append("Immediate follow-on:")
    for offset in range(1, 6):
        idx = divergence + offset
        if idx < len(passing_early):
            event = passing_early[idx]
            suffix = f" {event.detail}" if event.detail else ""
            lines.append(f"- passing+{offset}: {event.kind} {event.name}{suffix}")
        if idx < len(failing_early):
            event = failing_early[idx]
            suffix = f" {event.detail}" if event.detail else ""
            lines.append(f"- failing+{offset}: {event.kind} {event.name}{suffix}")

    return "\n".join(lines) + "\n"


def main() -> None:
    reset_dir(ARTIFACT_DIR)
    reset_dir(WORK_ROOT)

    good_dir = WORK_ROOT / "good"
    bad_dir = WORK_ROOT / "bad"
    tok_dir = WORK_ROOT / "bad-player-tok"
    good_dir.mkdir()
    bad_dir.mkdir()
    tok_dir.mkdir()

    prepare_good(good_dir)
    prepare_bad(bad_dir)
    prepare_bad_with_token(tok_dir, "PLAYER.TOK")

    good_log = ARTIFACT_DIR / "good.log"
    bad_log = ARTIFACT_DIR / "bad.log"
    tok_log = ARTIFACT_DIR / "bad-player-tok.log"
    run_trace(good_dir, good_log)
    run_trace(bad_dir, bad_log)
    run_trace(tok_dir, tok_log)

    good_events = parse_trace(good_log)
    bad_events = parse_trace(bad_log)
    tok_events = parse_trace(tok_log)

    good_error = None
    bad_error = None
    tok_error = None
    if (good_dir / "ERRORS.TXT").exists():
        good_error = (good_dir / "ERRORS.TXT").read_text(errors="ignore").splitlines()[0]
    if (bad_dir / "ERRORS.TXT").exists():
        bad_error = (bad_dir / "ERRORS.TXT").read_text(errors="ignore").splitlines()[0]
    if (tok_dir / "ERRORS.TXT").exists():
        tok_error = (tok_dir / "ERRORS.TXT").read_text(errors="ignore").splitlines()[0]

    write_summary("good-guard-starbase", good_events, ARTIFACT_DIR / "good-summary.txt", good_error)
    write_summary("bad-raw-starbase2-no-tok", bad_events, ARTIFACT_DIR / "bad-summary.txt", bad_error)
    write_summary("bad-raw-starbase2-player-tok", tok_events, ARTIFACT_DIR / "bad-player-tok-summary.txt", tok_error)
    (ARTIFACT_DIR / "comparison.txt").write_text(compare_events(good_events, bad_events))
    (ARTIFACT_DIR / "token-comparison.txt").write_text(compare_events(tok_events, bad_events))
    (ARTIFACT_DIR / "token-core-comparison.txt").write_text(compare_core_validation(tok_events, bad_events))

    print(f"Wrote {ARTIFACT_DIR / 'good.log'}")
    print(f"Wrote {ARTIFACT_DIR / 'bad.log'}")
    print(f"Wrote {ARTIFACT_DIR / 'bad-player-tok.log'}")
    print(f"Wrote {ARTIFACT_DIR / 'good-summary.txt'}")
    print(f"Wrote {ARTIFACT_DIR / 'bad-summary.txt'}")
    print(f"Wrote {ARTIFACT_DIR / 'bad-player-tok-summary.txt'}")
    print(f"Wrote {ARTIFACT_DIR / 'comparison.txt'}")
    print(f"Wrote {ARTIFACT_DIR / 'token-comparison.txt'}")
    print(f"Wrote {ARTIFACT_DIR / 'token-core-comparison.txt'}")


if __name__ == "__main__":
    main()
