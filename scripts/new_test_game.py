#!/usr/bin/env python3
from __future__ import annotations

import argparse
import shutil
import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_DIR = REPO_ROOT / "rust"


def run_ec_cli(*args: str) -> None:
    cmd = ["cargo", "run", "-q", "-p", "ec-cli", "--", *args]
    subprocess.run(cmd, cwd=RUST_DIR, check=True)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Create a fresh joinable test game backed by ecgame.db."
    )
    parser.add_argument("target_dir", help="Directory to create or replace.")
    parser.add_argument(
        "--players",
        type=int,
        required=True,
        help="Player count for the new game.",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=1515,
        help="Optional world-generation seed. Default: 1515.",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Remove the target directory first if it already exists.",
    )
    args = parser.parse_args()

    target = Path(args.target_dir).resolve()
    if target.exists():
        if not args.force:
            raise SystemExit(f"target already exists: {target} (use --force)")
        shutil.rmtree(target)

    run_ec_cli(
        "sysop",
        "new-game",
        str(target),
        "--players",
        str(args.players),
        "--seed",
        str(args.seed),
    )

    print()
    print(f"Created joinable test game at {target}")
    print(f"Players: {args.players}")
    print(f"Seed: {args.seed}")
    print("Launch with:")
    print(f"  python3 scripts/run_client.py {target} --player 1")


if __name__ == "__main__":
    main()
