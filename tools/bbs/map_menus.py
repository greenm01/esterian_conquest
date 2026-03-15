#!/tmp/venv/bin/python
import argparse
import json
import os
import re
import time
from dataclasses import dataclass

import pexpect
import pyte


def strip_telnet(data: bytes) -> bytes:
    clean_data = bytearray()
    i = 0
    while i < len(data):
        if data[i] == 255:
            if i + 1 < len(data) and data[i + 1] == 250:
                end_sb = data.find(bytes([255, 240]), i)
                i = end_sb + 2 if end_sb != -1 else i + 2
            elif i + 1 < len(data) and data[i + 1] in (251, 252, 253, 254):
                i += 3
            else:
                i += 2
        else:
            clean_data.append(data[i])
            i += 1
    return bytes(clean_data)


def render_display(raw: bytes) -> list[str]:
    screen = pyte.Screen(80, 25)
    stream = pyte.Stream(screen)
    stream.feed(strip_telnet(raw).decode("cp437", errors="ignore"))
    return list(screen.display)


def read_until_quiet(child: pexpect.spawn, settle_s: float = 0.8, total_s: float = 4.0) -> bytes:
    start = time.time()
    last = time.time()
    buf = bytearray()
    while time.time() - start < total_s:
        try:
            chunk = child.read_nonblocking(size=65535, timeout=0.2)
            if chunk:
                buf.extend(chunk)
                last = time.time()
        except pexpect.TIMEOUT:
            if time.time() - last >= settle_s:
                break
        except pexpect.EOF:
            break
    return bytes(buf)


@dataclass
class Step:
    name: str
    send: bytes | None = None
    settle_s: float = 1.0
    total_s: float = 4.0


POST_JOIN_SEQUENCE = [
    Step("main-menu"),
    Step("main-help", b"h\r"),
    Step("main-menu-return", b"\r"),
    Step("general-menu", b"g"),
    Step("general-help", b"h"),
    Step("general-menu-return", b"\r"),
    Step("general-status", b"s"),
    Step("general-menu-from-status", b"\r"),
    Step("general-enemies", b"e"),
    Step("general-menu-from-enemies", b"\r"),
    Step("general-report-review", b"r"),
    Step("general-menu-from-report-review", b"\r"),
    Step("general-other-empires", b"o"),
    Step("general-menu-from-other-empires", b"\r"),
    Step("main-menu-from-general", b"q"),
    Step("fleet-menu", b"f"),
    Step("fleet-help", b"h"),
    Step("fleet-menu-return", b"\r"),
    Step("fleet-brief-list", b"b"),
    Step("fleet-menu-from-brief-list", b"q"),
    Step("fleet-full-list", b"f"),
    Step("fleet-menu-from-full-list", b"q"),
    Step("fleet-review-prompt", b"r"),
    Step("fleet-menu-from-review-prompt", b"a\r", settle_s=1.0),
    Step("fleet-eta-prompt", b"e"),
    Step("fleet-menu-from-eta-prompt", b"a\r", settle_s=1.0),
    Step("fleet-change-roe-prompt", b"c"),
    Step("fleet-menu-from-change-roe-prompt", b"a\r", settle_s=1.0),
    Step("fleet-detach-prompt", b"d"),
    Step("fleet-menu-from-detach-prompt", b"a\r", settle_s=1.0),
    Step("fleet-planet-info-prompt", b"i"),
    Step("fleet-menu-from-planet-info-prompt", b"a\r", settle_s=1.0),
    Step("fleet-partial-map-prompt", b"v"),
    Step("fleet-menu-from-partial-map-prompt", b"a\r", settle_s=1.0),
    Step("main-menu-from-fleet", b"q"),
    Step("planet-menu", b"p"),
    Step("planet-help", b"h"),
    Step("planet-menu-return", b"\r"),
    Step("planet-brief-list", b"p"),
    Step("planet-menu-from-brief-list", b"q"),
    Step("planet-detail-list", b"d"),
    Step("planet-menu-from-detail-list", b"q"),
    Step("planet-build-menu", b"b"),
    Step("planet-menu-from-build-menu", b"q"),
    Step("planet-tax-prompt", b"t"),
    Step("planet-menu-from-tax-prompt", b"a\r", settle_s=1.0),
    Step("planet-info-prompt", b"i"),
    Step("planet-menu-from-info-prompt", b"a\r", settle_s=1.0),
    Step("planet-partial-map-prompt", b"v"),
    Step("planet-menu-from-partial-map-prompt", b"a\r", settle_s=1.0),
    Step("main-menu-from-planet", b"q"),
]

FIRST_TIME_SEQUENCE = [
    Step("first-time-menu"),
    Step("first-time-help", b"h"),
    Step("first-time-menu-return", b"\r"),
    Step("first-time-list-empires", b"l"),
    Step("first-time-menu-from-list-empires", b"\r"),
    Step("first-time-view-intro", b"v"),
    Step("first-time-menu-from-view-intro", b"\r"),
]

JOIN_SEQUENCE = [
    Step("first-time-menu"),
    Step("join-prompt", b"j"),
    Step("join-select-player", b"1\r", settle_s=1.0),
    Step("join-enter-empire-name", b"Mag Test Empire\r", settle_s=1.0),
    Step("join-enter-homeworld-name", b"Mag Prime\r", settle_s=1.0),
    Step("post-join-main-menu", b"\r", settle_s=1.2),
]

SCENARIOS = {
    "post-join": POST_JOIN_SEQUENCE,
    "first-time": FIRST_TIME_SEQUENCE,
    "join": JOIN_SEQUENCE,
}


def login_and_enter_game(child: pexpect.spawn, user: str, password: str) -> None:
    try:
        child.expect(b"version", timeout=5)
    except pexpect.TIMEOUT:
        pass
    time.sleep(2)
    child.send(b"\r")
    time.sleep(2)
    child.send(user.encode("ascii") + b"\r")
    time.sleep(1)
    child.send(password.encode("ascii") + b"\r")
    time.sleep(1)
    child.send(b"\r")
    child.expect(b"Y/\\[N\\]", timeout=10)
    child.send(b"Y\r")
    child.expect(b"\\(Press Return\\)", timeout=15)
    child.send(b"\r")


def capture_sequence(host: str, port: int, user: str, password: str) -> list[dict]:
    child = pexpect.spawn(f"telnet {host} {port}", timeout=15)
    child.delaybeforesend = 0.05
    try:
        login_and_enter_game(child, user, password)
        results = []
        for index, step in enumerate(SEQUENCE):
            if step.send:
                child.send(step.send)
            raw = read_until_quiet(child, settle_s=step.settle_s, total_s=step.total_s)
            display = render_display(raw)
            results.append(
                {
                    "index": index,
                    "name": step.name,
                    "sent": step.send.decode("ascii", errors="ignore") if step.send else "",
                    "lines": display,
                }
            )
        return results
    finally:
        child.close(force=True)


def write_outputs(results: list[dict], out_dir: str) -> None:
    os.makedirs(out_dir, exist_ok=True)
    with open(os.path.join(out_dir, "menu_capture.json"), "w", encoding="utf-8") as handle:
        json.dump(results, handle, indent=2)
    for result in results:
        filename = f"{result['index']:02d}-{result['name']}.txt"
        with open(os.path.join(out_dir, filename), "w", encoding="utf-8") as handle:
            handle.write("\n".join(result["lines"]))
            handle.write("\n")


def append_step_output(result: dict, out_dir: str) -> None:
    os.makedirs(out_dir, exist_ok=True)
    filename = f"{result['index']:02d}-{result['name']}.txt"
    with open(os.path.join(out_dir, filename), "w", encoding="utf-8") as handle:
        handle.write("\n".join(result["lines"]))
        handle.write("\n")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8888)
    parser.add_argument("--user", default="mag")
    parser.add_argument("--password", default="fooBar")
    parser.add_argument("--out-dir", default="capture/bbs-menu-map")
    parser.add_argument("--scenario", choices=sorted(SCENARIOS), default="post-join")
    parser.add_argument("--start", type=int, default=0)
    parser.add_argument("--stop", type=int, default=None)
    args = parser.parse_args()

    sequence = SCENARIOS[args.scenario]
    selected = sequence[args.start : args.stop]
    if not selected:
        raise SystemExit("selected step range is empty")
    child = pexpect.spawn(f"telnet {args.host} {args.port}", timeout=15)
    child.delaybeforesend = 0.05
    results = []
    try:
        login_and_enter_game(child, args.user, args.password)
        for offset, step in enumerate(selected, start=args.start):
            if step.send:
                child.send(step.send)
            raw = read_until_quiet(child, settle_s=step.settle_s, total_s=step.total_s)
            display = render_display(raw)
            result = {
                "index": offset,
                "name": step.name,
                "sent": step.send.decode("ascii", errors="ignore") if step.send else "",
                "lines": display,
            }
            results.append(result)
            append_step_output(result, args.out_dir)
            print(f"captured step {offset:02d} {step.name}")
    finally:
        child.close(force=True)
    write_outputs(results, args.out_dir)
    for result in results:
        print(f"== {result['index']:02d} {result['name']} ==")
        for line in result["lines"]:
            print(line.rstrip())
        print()


if __name__ == "__main__":
    main()
