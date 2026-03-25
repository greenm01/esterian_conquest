#!/usr/bin/env python3
from __future__ import annotations

import argparse
import shutil
import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_DIR = REPO_ROOT / "rust"
DEFAULT_SEED = 1515

PLAYER_SPECS = [
    ("p1", "Aurora", 55),
    ("p2", "Red Horizon Pact", 48),
    ("p3", "Vela Syndicate", 42),
    ("p4", "Helios Crown", 60),
    ("p5", "Obsidian Reach", 47),
    ("p6", "Silver Spiral", 52),
    ("p7", "Cinder Bloc", 44),
    ("p8", "Pale Meridian", 50),
    ("p9", "Viridian Chain", 46),
    ("p10", "Iron Comet", 58),
    ("p11", "Glass Dominion", 49),
    ("p12", "Sable Choir", 53),
]


def run_ec_cli(*args: str) -> None:
    subprocess.run(
        ["cargo", "run", "-q", "-p", "ec-cli", "--", *args],
        cwd=RUST_DIR,
        check=True,
    )


def set_player(target: Path, record: int, handle: str, empire: str, tax: int) -> None:
    run_ec_cli("player-name", str(target), str(record), handle, empire)
    run_ec_cli("player-tax", str(target), str(record), str(tax))


def map_size_for_player_count(player_count: int) -> int:
    if player_count <= 4:
        return 18
    if player_count <= 9:
        return 27
    if player_count <= 16:
        return 36
    return 45


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Create a player-1-focused TUI stress-test game."
    )
    parser.add_argument("target_dir", help="Directory to create or replace.")
    parser.add_argument(
        "--players",
        type=int,
        default=12,
        help="Player count for this stress template. Supported range: 4-12. Default: 12.",
    )
    parser.add_argument(
        "--turn",
        type=int,
        default=1,
        help="Target turn number after setup. Turn 1 is the seeded baseline; higher turns run maint-rust repeatedly. Default: 1.",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=DEFAULT_SEED,
        help=f"Map-generation seed for the engine-backed new-game setup. Default: {DEFAULT_SEED}.",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Remove the target directory first if it already exists.",
    )
    args = parser.parse_args()

    if args.turn < 1:
        raise SystemExit("--turn must be >= 1")
    if not 4 <= args.players <= len(PLAYER_SPECS):
        raise SystemExit(f"--players must be between 4 and {len(PLAYER_SPECS)}")
    if args.seed < 0:
        raise SystemExit("--seed must be >= 0")

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

    for idx, (handle, empire, tax) in enumerate(PLAYER_SPECS[: args.players], start=1):
        set_player(target, idx, handle, empire, tax)

    run_ec_cli("harness", "seed-player1-tui-stress", "--dir", str(target))
    for _ in range(args.turn - 1):
        run_ec_cli("maint-rust", str(target), "1")

    print()
    print(f"Created player-1 TUI stress game at {target}")
    print(f"Turn: {args.turn}")
    print(f"Players: {args.players}")
    print(f"Seed: {args.seed}")
    print(f"Map size: {map_size_for_player_count(args.players)}x{map_size_for_player_count(args.players)}")
    print("Player 1 extras: active starbases, messages, reports, mixed foreign intel")
    print("Launch with:")
    print(f"  python3 scripts/run_client.py {target} --player 1")


if __name__ == "__main__":
    main()
