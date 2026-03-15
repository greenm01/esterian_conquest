#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_DIR = REPO_ROOT / "rust"


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Launch the Rust EC client for a chosen campaign directory and player seat."
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
    args = parser.parse_args()

    game_dir = Path(args.game_dir).resolve()
    cmd = ["cargo", "run"]
    if args.release:
        cmd.append("--release")
    cmd.extend(
        [
            "-q",
            "-p",
            "ec-client",
            "--",
            "--dir",
            str(game_dir),
            "--player",
            str(args.player),
        ]
    )
    subprocess.run(cmd, cwd=RUST_DIR, check=True)


if __name__ == "__main__":
    main()
