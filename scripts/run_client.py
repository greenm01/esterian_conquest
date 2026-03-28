#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_DIR = REPO_ROOT / "rust"


def cargo_profile_dir(release: bool) -> str:
    return "release" if release else "debug"


def build_workspace_binary(package: str, release: bool) -> Path:
    cmd = ["cargo", "build", "-q", "-p", package]
    if release:
        cmd.insert(2, "--release")
    subprocess.run(cmd, cwd=RUST_DIR, check=True)
    return RUST_DIR / "target" / cargo_profile_dir(release) / package


def refresh_campaign_snapshot(game_dir: Path, release: bool) -> None:
    cli_binary = build_workspace_binary("ec-cli", release)
    result = subprocess.run(
        [str(cli_binary), "db-import", str(game_dir)],
        cwd=RUST_DIR,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        if result.stdout:
            sys.stderr.write(result.stdout)
        if result.stderr:
            sys.stderr.write(result.stderr)
        raise subprocess.CalledProcessError(
            result.returncode,
            result.args,
            output=result.stdout,
            stderr=result.stderr,
        )


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Launch the Rust ec-game client for a chosen campaign directory and player seat."
    )
    parser.add_argument("game_dir", help="Campaign directory that contains ecgame.db.")
    parser.add_argument(
        "--player",
        type=int,
        default=1,
        help="Player seat to launch. Default: 1.",
    )
    parser.add_argument(
        "--release",
        action="store_true",
        help="Run the release build instead of debug.",
    )
    parser.add_argument(
        "--refresh-from-dat",
        action="store_true",
        help=(
            "Re-import classic .DAT files into ecgame.db before launch. "
            "Use this only when you intentionally want the runtime DB to be refreshed "
            "from classic files."
        ),
    )
    parser.add_argument(
        "--log-file",
        default=None,
        metavar="PATH",
        help="Write ec-game logs to this file.",
    )
    parser.add_argument(
        "--log-level",
        default=None,
        metavar="LEVEL",
        help="Log level to pass to ec-game: error, warn, info, debug, or trace.",
    )
    args = parser.parse_args()

    game_dir = Path(args.game_dir).resolve()
    if args.refresh_from_dat:
        refresh_campaign_snapshot(game_dir, args.release)
    client_binary = build_workspace_binary("ec-game", args.release)
    cmd = [
        str(client_binary),
        "--dir",
        str(game_dir),
        "--player",
        str(args.player),
    ]
    if args.log_file:
        cmd += ["--log-file", args.log_file]
    if args.log_level:
        cmd += ["--log-level", args.log_level]
    subprocess.run(cmd, cwd=RUST_DIR, check=True)


if __name__ == "__main__":
    main()
