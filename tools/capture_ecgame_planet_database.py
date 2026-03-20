#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import os
import time
from pathlib import Path

from ecgame_dropfiles import write_chain_txt
from pexpect_argv import spawn_argv


DEFAULT_KEYS = ["T", "ENTER", "ENTER"]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Capture classic ECGAME Total Planet Database screens and DATABASE.DAT rewrites."
    )
    parser.add_argument("game_dir", type=Path, help="Prepared classic game directory")
    parser.add_argument("--player", type=int, default=1, help="Classic player number")
    parser.add_argument("--alias", default="SYSOP", help="Caller alias for CHAIN.TXT")
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("/tmp/ecgame-planet-database"),
        help="Directory for captured screens and DATABASE.DAT snapshots",
    )
    parser.add_argument(
        "--keys",
        default=",".join(DEFAULT_KEYS),
        help="Comma-separated key sequence after startup (default enters Total Planet Database and accepts the first two prompts).",
    )
    return parser.parse_args()


def decode_b800_screen(data: bytes) -> str:
    cells = bytearray()
    for idx in range(0, min(len(data), 4000), 2):
        cells.append(data[idx])
    text = cells.decode("cp437", errors="replace")
    lines = [text[row : row + 80].rstrip() for row in range(0, len(text), 80)]
    while lines and not lines[-1]:
        lines.pop()
    return "\n".join(lines) + ("\n" if lines else "")


def dump_screen(child, game_dir: Path, output_dir: Path, step: int) -> str:
    output_dir.mkdir(parents=True, exist_ok=True)
    dump_path = game_dir / "MEMDUMP.BIN"
    if dump_path.exists():
        dump_path.unlink()
    child.sendline("MEMDUMPBIN B800:0000 4000")
    wait_for_prompt(child)
    for _ in range(20):
        if dump_path.exists():
            break
        time.sleep(0.1)
    if not dump_path.exists():
        placeholder = "<screen dump unavailable>\n"
        (output_dir / f"screen_{step:02}.txt").write_text(placeholder, encoding="utf-8")
        return placeholder
    data = dump_path.read_bytes()
    raw_path = output_dir / f"screen_{step:02}.bin"
    text_path = output_dir / f"screen_{step:02}.txt"
    raw_path.write_bytes(data)
    decoded = decode_b800_screen(data)
    text_path.write_text(decoded, encoding="utf-8")
    dump_path.unlink()
    return decoded


def normalize_key(token: str) -> str:
    token = token.strip()
    upper = token.upper()
    if upper in {"ENTER", "RETURN"}:
        return "\r"
    if upper == "SPACE":
        return " "
    if upper == "TAB":
        return "\t"
    return token


def wait_for_prompt(child) -> bool:
    try:
        child.expect([r"I-> _", r"I->", r"> _", r"DBG>", r"CS="], timeout=5)
        time.sleep(0.1)
        return True
    except Exception:
        return False


def snapshot_database(game_dir: Path, output_dir: Path, label: str) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    database_path = game_dir / "DATABASE.DAT"
    bytes_out = output_dir / f"database_{label}.dat"
    if database_path.exists():
        data = database_path.read_bytes()
        bytes_out.write_bytes(data)
        digest = hashlib.sha256(data).hexdigest()
        (output_dir / f"database_{label}.sha256").write_text(f"{digest}\n", encoding="utf-8")


def main() -> None:
    args = parse_args()
    game_dir = args.game_dir.resolve()
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    write_chain_txt(
        game_dir / "CHAIN.TXT",
        player_number=args.player,
        alias=args.alias,
        real_name=args.alias,
    )
    snapshot_database(game_dir, output_dir, "before")

    exe_name = "ECGAME.EXE" if (game_dir / "ECGAME.EXE").exists() else "ECGAME"
    cmd = [
        "dosbox-x",
        "-defaultconf",
        "-nopromptfolder",
        "-nogui",
        "-nomenu",
        "-defaultdir",
        str(game_dir),
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
        f"mount c {game_dir}",
        "-c",
        "c:",
        "-c",
        "mode co80",
        "-c",
        f"DEBUGBOX {exe_name}",
    ]

    env = os.environ.copy()
    env.setdefault("SDL_VIDEODRIVER", "dummy")
    env.setdefault("SDL_AUDIODRIVER", "dummy")
    env.setdefault("TERM", "dumb")

    child = spawn_argv(cmd, env=env, timeout=20, encoding="cp437")
    log_path = output_dir / "debugbox.log"
    log_file = log_path.open("w", encoding="utf-8", errors="replace")
    child.logfile = log_file
    transcript: list[str] = []
    try:
        wait_for_prompt(child)
        child.sendline("BPINT 16 00")
        wait_for_prompt(child)

        child.sendline("RUN")
        time.sleep(3)
        wait_for_prompt(child)
        transcript.append("=== screen_00 ===\n")
        transcript.append(dump_screen(child, game_dir, output_dir, 0))

        keys = [normalize_key(token) for token in args.keys.split(",") if token.strip()]
        for step, key in enumerate(keys, start=1):
            child.sendline("RUN")
            time.sleep(0.2)
            child.send(key)
            time.sleep(3)
            wait_for_prompt(child)
            transcript.append(
                f"\n=== screen_{step:02} key={key.encode('unicode_escape').decode()} ===\n"
            )
            transcript.append(dump_screen(child, game_dir, output_dir, step))

        snapshot_database(game_dir, output_dir, "after")
        (output_dir / "transcript.txt").write_text("".join(transcript), encoding="utf-8")

        child.sendline("EXIT")
        time.sleep(1)
        child.close()
        print(f"Captured planet-database probe artifacts in {output_dir}")
    finally:
        if child.isalive():
            child.close(force=True)
        log_file.close()
        if (game_dir / "DATABASE.DAT").exists() and not (output_dir / "database_after.dat").exists():
            snapshot_database(game_dir, output_dir, "after")


if __name__ == "__main__":
    main()
