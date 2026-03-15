#!/usr/bin/env python3
from __future__ import annotations

import argparse
import shutil
import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_DIR = REPO_ROOT / "rust"

HOMEWORLD_COORDS = ["16:13", "30:6", "2:25", "26:26"]
PLAYER_SPECS = [
    ("mag", "Auroran Combine", 45),
    ("trent", "Red Horizon Pact", 50),
    ("iris", "Vela Syndicate", 35),
    ("nora", "Helios Crown", 60),
]
PLANET_SPECS = [
    (1, 1, "Aurora Prime", 140, 35, 10, 7),
    (2, 1, "Anvil", 110, 22, 6, 4),
    (3, 1, "Lattice", 96, 14, 4, 2),
    (4, 1, "Cobalt", 88, 10, 3, 2),
    (5, 1, "Relay", 72, 8, 2, 1),
    (6, 2, "Red Haven", 132, 30, 9, 6),
    (7, 2, "Furnace", 104, 18, 5, 3),
    (8, 2, "Signal", 84, 11, 3, 2),
    (9, 2, "Basilisk", 90, 9, 3, 1),
    (10, 2, "Harbor", 68, 6, 2, 1),
    (11, 3, "Vela Prime", 128, 28, 8, 5),
    (12, 3, "Prospect", 98, 16, 4, 2),
    (13, 3, "Whisper", 82, 12, 3, 2),
    (14, 3, "Gloam", 76, 9, 2, 1),
    (15, 3, "Spindle", 66, 5, 1, 0),
    (16, 4, "Helios Prime", 136, 32, 10, 6),
    (17, 4, "Bastion", 106, 20, 5, 3),
    (18, 4, "Mercy", 86, 13, 4, 2),
    (19, 4, "Chalice", 78, 9, 2, 1),
    (20, 4, "Anchor", 64, 7, 2, 1),
]
P1_DETACH_SPECS = [
    (0, 0, 1, 0, 0, 1, 0, 2),
    (0, 1, 0, 0, 0, 1, 0, 3),
    (1, 0, 0, 0, 0, 0, 0, 4),
    (0, 1, 1, 0, 0, 0, 0, 5),
    (1, 1, 0, 0, 0, 0, 0, 6),
    (0, 0, 2, 0, 0, 0, 0, 7),
    (0, 0, 1, 1, 0, 0, 0, 8),
    (0, 1, 0, 1, 0, 0, 0, 9),
    (0, 0, 0, 0, 1, 0, 0, 10),
    (0, 0, 0, 1, 0, 0, 0, 11),
    (0, 0, 0, 1, 0, 1, 0, 12),
    (1, 0, 1, 0, 0, 0, 0, 13),
    (0, 2, 0, 0, 0, 0, 0, 14),
    (0, 1, 0, 0, 1, 0, 0, 15),
    (1, 0, 0, 0, 1, 0, 0, 1),
    (0, 0, 1, 1, 0, 1, 0, 2),
    (0, 1, 1, 0, 0, 1, 0, 3),
    (1, 0, 0, 1, 0, 0, 0, 4),
    (0, 0, 0, 0, 0, 2, 0, 5),
    (0, 0, 0, 0, 0, 1, 1, 6),
    (0, 0, 1, 0, 0, 0, 1, 7),
    (0, 1, 0, 0, 0, 0, 1, 8),
    (1, 0, 0, 0, 0, 1, 0, 9),
    (0, 0, 1, 0, 1, 0, 0, 10),
    (0, 1, 0, 0, 0, 0, 0, 11),
    (0, 0, 1, 0, 0, 0, 0, 12),
]
OTHER_DETACH_SPECS = [
    (0, 1, 0, 0, 0, 0, 0, 3),
    (0, 0, 1, 0, 0, 0, 0, 5),
    (1, 0, 0, 0, 0, 0, 0, 7),
    (0, 0, 0, 1, 0, 0, 0, 9),
]


def run_ec_cli(*args: str) -> None:
    subprocess.run(
        ["cargo", "run", "-q", "-p", "ec-cli", "--", *args],
        cwd=RUST_DIR,
        check=True,
    )


def set_planet(target: Path, record: int, owner: int, name: str, potential: int, stored: int, armies: int, batteries: int) -> None:
    run_ec_cli("planet-owner", str(target), str(record), str(owner))
    run_ec_cli("planet-name", str(target), str(record), name)
    run_ec_cli("planet-potential", str(target), str(record), str(potential), "0")
    run_ec_cli("planet-stored", str(target), str(record), str(stored))
    run_ec_cli("planet-stats", str(target), str(record), str(armies), str(batteries))


def fleet_order_for(target: Path, fleet_record: int, order_code: int, x: int, y: int, speed: int) -> None:
    run_ec_cli(
        "fleet-order",
        str(target),
        str(fleet_record),
        str(speed),
        str(order_code),
        str(x),
        str(y),
    )


def configure_player_one_fleets(target: Path) -> None:
    run_ec_cli("fleet-ships", str(target), "1", "45", "18", "26", "40", "18", "8", "6")
    run_ec_cli("fleet-ships", str(target), "2", "6", "4", "8", "10", "4", "2", "0")
    run_ec_cli("fleet-ships", str(target), "3", "10", "2", "6", "8", "0", "0", "0")
    run_ec_cli("fleet-ships", str(target), "4", "12", "0", "4", "4", "2", "0", "2")

    for offset, spec in enumerate(P1_DETACH_SPECS, start=17):
        bb, ca, dd, full_tt, empty_tt, scouts, etacs, roe = spec
        run_ec_cli(
            "fleet-detach",
            str(target),
            "1",
            "1",
            str(bb),
            str(ca),
            str(dd),
            str(full_tt),
            str(empty_tt),
            str(scouts),
            str(etacs),
            "3",
            str(roe),
        )
        target_x = ((offset * 3) % 38) + 1
        target_y = ((offset * 5) % 38) + 1
        order_code = [0, 1, 3, 9, 10, 11, 14][offset % 7]
        speed = 1 + (offset % 3)
        fleet_order_for(target, offset, order_code, target_x, target_y, speed)

    for record, order_code, coords, speed in [
        (1, 14, (15, 13), 3),
        (2, 3, (18, 15), 2),
        (3, 10, (20, 8), 3),
        (4, 11, (25, 14), 2),
    ]:
        fleet_order_for(target, record, order_code, coords[0], coords[1], speed)


def configure_other_player_fleets(target: Path, player: int, donor_record: int) -> None:
    base_counts = {
        2: ("8", "6", "10", "14", "6", "2", "1"),
        3: ("12", "4", "8", "12", "6", "2", "0"),
        4: ("5", "8", "12", "10", "5", "1", "2"),
    }
    run_ec_cli("fleet-ships", str(target), str(donor_record), *base_counts[player])
    for idx, spec in enumerate(OTHER_DETACH_SPECS):
        bb, ca, dd, full_tt, empty_tt, scouts, etacs, roe = spec
        run_ec_cli(
            "fleet-detach",
            str(target),
            str(player),
            str(donor_record),
            str(bb),
            str(ca),
            str(dd),
            str(full_tt),
            str(empty_tt),
            str(scouts),
            str(etacs),
            "2",
            str(roe),
        )
        new_record = 42 + ((player - 2) * 4) + idx + 1
        order_code = [1, 3, 10, 14][idx]
        x = 4 + (player * 7) + idx
        y = 6 + (player * 4) + idx
        fleet_order_for(target, new_record, order_code, x, y, 2)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Create a four-player year-3010 UI/maintenance stress-test game."
    )
    parser.add_argument("target_dir", help="Directory to create or replace.")
    parser.add_argument(
        "--seed",
        type=int,
        default=3010,
        help="World-generation seed. Default: 3010.",
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
        "generate-gamestate",
        str(target),
        "4",
        "3010",
        *HOMEWORLD_COORDS,
    )

    for idx, (handle, empire, tax) in enumerate(PLAYER_SPECS, start=1):
        run_ec_cli("player-name", str(target), str(idx), handle, empire)
        run_ec_cli("player-tax", str(target), str(idx), str(tax))

    for record, owner, name, potential, stored, armies, batteries in PLANET_SPECS:
        set_planet(target, record, owner, name, potential, stored, armies, batteries)

    run_ec_cli("planet-stardock", str(target), "1", "0", "4", "2")
    run_ec_cli("planet-stardock", str(target), "6", "0", "4", "1")
    run_ec_cli("planet-stardock", str(target), "11", "0", "4", "1")
    run_ec_cli("planet-stardock", str(target), "16", "0", "4", "2")

    configure_player_one_fleets(target)
    configure_other_player_fleets(target, 2, 5)
    configure_other_player_fleets(target, 3, 9)
    configure_other_player_fleets(target, 4, 13)

    print()
    print(f"Created UI stress-test game at {target}")
    print("Year: 3010")
    print("Players: 4")
    print("Player 1 fleets: 30")
    print("Players 2-4 fleets: 8 each")
    print("Launch with:")
    print(f"  python3 scripts/run_client.py {target} --player 1")


if __name__ == "__main__":
    main()
