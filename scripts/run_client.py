#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
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
    subprocess.run([str(cli_binary), "db-import", str(game_dir)], cwd=RUST_DIR, check=True)


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
    refresh_campaign_snapshot(game_dir, args.release)
    client_binary = build_workspace_binary("ec-client", args.release)
    subprocess.run(
        [
            str(client_binary),
            "--dir",
            str(game_dir),
            "--player",
            str(args.player),
        ],
        cwd=RUST_DIR,
        check=True,
    )


if __name__ == "__main__":
    main()
