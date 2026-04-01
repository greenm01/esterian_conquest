#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import tempfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_DIR = REPO_ROOT / "rust"
DEFAULT_SEED = 1515
DEFAULT_YEAR = 3000
DEFAULT_RELAY_URL = "ws://localhost:8080"
DEFAULT_SERVER_HOST = "localhost"
DEFAULT_SERVER_PORT = 22
DEFAULT_GATE_STATE_DIR = Path("/tmp/ec-local-gate")
DEFAULT_EC_CONNECT_PASSWORD = "testing"

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


def run_nc_cli(*args: str) -> None:
    env = os.environ.copy()
    env["RUSTC_WRAPPER"] = ""
    subprocess.run(
        ["cargo", "run", "-q", "-p", "nc-cli", "--", *args],
        cwd=RUST_DIR,
        check=True,
        env=env,
    )


def run_nc_sysop(*args: str) -> str:
    env = os.environ.copy()
    env["RUSTC_WRAPPER"] = ""
    result = subprocess.run(
        ["cargo", "run", "-q", "-p", "nc-sysop", "--", *args],
        cwd=RUST_DIR,
        check=True,
        env=env,
        text=True,
        capture_output=True,
    )
    return result.stdout


def run_nc_connect(*args: str) -> str:
    env = os.environ.copy()
    env["RUSTC_WRAPPER"] = ""
    result = subprocess.run(
        ["cargo", "run", "-q", "-p", "nc-connect", "--bin", "nc-connect", "--", *args],
        cwd=RUST_DIR,
        check=True,
        env=env,
        text=True,
        capture_output=True,
    )
    return result.stdout


def set_player(target: Path, record: int, handle: str, empire: str, tax: int) -> None:
    run_nc_cli("player-name", str(target), str(record), handle, empire)
    run_nc_cli("player-tax", str(target), str(record), str(tax))


def map_size_for_player_count(player_count: int) -> int:
    if player_count <= 4:
        return 18
    if player_count <= 9:
        return 27
    if player_count <= 16:
        return 36
    return 45


def parse_npub(stdout: str) -> str:
    for line in stdout.splitlines():
        if line.startswith("Public key (npub): "):
            return line.split(": ", 1)[1].strip()
        if line.startswith("player npub: "):
            return line.split(": ", 1)[1].strip()
    raise SystemExit(f"unable to parse npub from command output:\n{stdout}")


def fixture_paths(root: Path) -> tuple[Path, Path, Path]:
    wallet = root / "data" / "nc" / "wallet.kdl"
    cache = root / "data" / "nc" / "cache.kdl"
    config = root / "config" / "nc" / "config.kdl"
    return wallet, cache, config


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
        "--year",
        type=int,
        default=DEFAULT_YEAR,
        help=f"Starting campaign year for the engine-backed new-game setup. Default: {DEFAULT_YEAR}.",
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
    parser.add_argument(
        "--hosted-claim-player",
        type=int,
        help="Pre-claim one hosted seat for a returning-player localhost fixture.",
    )
    parser.add_argument(
        "--hosted-nsec-file",
        help="Path to the nsec file used for the returning-player localhost fixture.",
    )
    parser.add_argument(
        "--nc-connect-data-root",
        help="Isolated XDG-like root for the seeded localhost nc-connect fixture.",
    )
    parser.add_argument(
        "--nc-connect-password",
        default=DEFAULT_EC_CONNECT_PASSWORD,
        help=f"Password for the isolated localhost nc-connect wallet. Default: {DEFAULT_EC_CONNECT_PASSWORD}.",
    )
    parser.add_argument(
        "--localhost-gate-state-dir",
        default=str(DEFAULT_GATE_STATE_DIR),
        help=f"State dir used for the localhost gate identity. Default: {DEFAULT_GATE_STATE_DIR}.",
    )
    args = parser.parse_args()

    if args.turn < 1:
        raise SystemExit("--turn must be >= 1")
    if not 4 <= args.players <= len(PLAYER_SPECS):
        raise SystemExit(f"--players must be between 4 and {len(PLAYER_SPECS)}")
    if not 0 <= args.year <= 65535:
        raise SystemExit("--year must be between 0 and 65535")
    if args.seed < 0:
        raise SystemExit("--seed must be >= 0")
    if args.hosted_claim_player is not None:
        if not 1 <= args.hosted_claim_player <= args.players:
            raise SystemExit("--hosted-claim-player must be within the seeded player range")
        if not args.hosted_nsec_file:
            raise SystemExit("--hosted-nsec-file is required with --hosted-claim-player")
    elif args.hosted_nsec_file:
        raise SystemExit("--hosted-nsec-file requires --hosted-claim-player")

    target = Path(args.target_dir).resolve()
    if target.exists():
        if not args.force:
            raise SystemExit(f"target already exists: {target} (use --force)")
        shutil.rmtree(target)

    run_nc_sysop(
        "new-game",
        str(target),
        "--players",
        str(args.players),
        "--year",
        str(args.year),
        "--seed",
        str(args.seed),
    )

    for idx, (handle, empire, tax) in enumerate(PLAYER_SPECS[: args.players], start=1):
        set_player(target, idx, handle, empire, tax)

    run_nc_cli("harness", "seed-player1-tui-stress", "--dir", str(target))
    for _ in range(args.turn - 1):
        run_nc_cli("maint-rust", str(target), "1")

    print()
    print(f"Created player-1 TUI stress game at {target}")
    print(f"Start year: {args.year}")
    print(f"Current year: {args.year + args.turn - 1}")
    print(f"Turn: {args.turn}")
    print(f"Players: {args.players}")
    print(f"Seed: {args.seed}")
    print(f"Map size: {map_size_for_player_count(args.players)}x{map_size_for_player_count(args.players)}")
    print("Player 1 extras: active starbases, messages, reports, mixed foreign intel")
    print("Launch with:")
    print(f"  python3 scripts/run_client.py {target} --player 1")

    if args.hosted_claim_player is not None:
        gate_state_dir = Path(args.localhost_gate_state_dir).resolve()
        gate_state_dir.mkdir(parents=True, exist_ok=True)
        gate_identity = gate_state_dir / "identity.kdl"
        gate_stdout = run_nc_sysop(
            "nostr",
            "init",
            "--identity",
            str(gate_identity),
        )
        gate_npub = parse_npub(gate_stdout)

        data_root = (
            Path(args.nc_connect_data_root).resolve()
            if args.nc_connect_data_root
            else Path(tempfile.gettempdir()) / f"nc-connect-localhost-{target.name}"
        )
        wallet_out, cache_out, config_out = fixture_paths(data_root)
        seed_args = [
            "dev",
            "seed-localhost-fixture",
            "--nsec-file",
            str(Path(args.hosted_nsec_file).resolve()),
            "--wallet-out",
            str(wallet_out),
            "--cache-out",
            str(cache_out),
            "--config-out",
            str(config_out),
            "--relay",
            DEFAULT_RELAY_URL,
            "--game-id",
            target.name,
            "--game-name",
            "Player 1 TUI Stress",
            "--player-name",
            PLAYER_SPECS[args.hosted_claim_player - 1][1],
            "--server",
            DEFAULT_SERVER_HOST,
            "--port",
            str(DEFAULT_SERVER_PORT),
            "--seat",
            str(args.hosted_claim_player),
            "--gate-npub",
            gate_npub,
            "--password",
            args.nc_connect_password,
        ]
        if args.force:
            seed_args.append("--force")
        fixture_stdout = run_nc_connect(*seed_args)
        player_npub = parse_npub(fixture_stdout)
        run_nc_sysop(
            "nostr",
            "claim",
            "--dir",
            str(target),
            "--player",
            str(args.hosted_claim_player),
            "--npub",
            player_npub,
        )

        print()
        print("Returning-player localhost fixture:")
        print(f"Claimed seat: {args.hosted_claim_player}")
        print(f"Player npub: {player_npub}")
        print(f"Gate npub: {gate_npub}")
        print(f"nc-connect data root: {data_root}")
        print(f"nc-connect password: {args.nc_connect_password}")
        print("Start localhost hosting with:")
        print(f"  ./scripts/start_local_gui_hosted_test.sh --dir {target}")
        print("Then launch nc-connect against the isolated fixture state with:")
        print(
            "  "
            f"XDG_CONFIG_HOME={data_root / 'config'} "
            f"XDG_DATA_HOME={data_root / 'data'} "
            "cargo run -q -p nc-connect --bin nc-connect"
        )
        print("This fixture opens as a returning player from the picker; it does not exercise the pending-seat first-join flow.")


if __name__ == "__main__":
    main()
