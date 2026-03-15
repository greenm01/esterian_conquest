#!/usr/bin/env python3
from __future__ import annotations

import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

from ecgame_dropfiles import write_chain_txt


ROOT = Path("/tmp/ecgame-u8-unit-limits")
FIXTURE = Path("fixtures/ecutil-init/v1.5")
ORIGINAL = Path("original/v1.5")

PLANET_RECORD_SIZE = 97
FLEET_RECORD_SIZE = 54
HOMEWORLD_PLANET_INDEX = 15  # 1-based; player 1 homeworld at (16,13)
HOMEWORLD_FLEET_INDEX = 1  # 1-based; player 1 fleet at homeworld
HOMEWORLD_COORDS = (16, 13)


def reset_dir(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)
    shutil.copytree(FIXTURE, path)
    shutil.copy2(ORIGINAL / "ECGAME.EXE", path)
    shutil.copy2(ORIGINAL / "ECMAINT.EXE", path)
    write_chain_txt(path / "CHAIN.TXT", player_number=1)


def patch_bytes(path: Path, offset: int, data: bytes) -> None:
    with path.open("r+b") as handle:
        handle.seek(offset)
        handle.write(data)


def patch_u8(path: Path, offset: int, value: int) -> None:
    patch_bytes(path, offset, bytes([value & 0xFF]))


def patch_u16(path: Path, offset: int, value: int) -> None:
    patch_bytes(path, offset, int(value).to_bytes(2, "little"))


def planet_offset(record_index_1_based: int, field_offset: int) -> int:
    return (record_index_1_based - 1) * PLANET_RECORD_SIZE + field_offset


def fleet_offset(record_index_1_based: int, field_offset: int) -> int:
    return (record_index_1_based - 1) * FLEET_RECORD_SIZE + field_offset


def run_ecmaint(target: Path) -> None:
    cmd = [
        "dosbox-x",
        "-defaultconf",
        "-nopromptfolder",
        "-nogui",
        "-nomenu",
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
        f"mount c {target}",
        "-c",
        "c:",
        "-c",
        "ECMAINT.EXE",
        "-c",
        "exit",
    ]
    subprocess.run(
        cmd,
        check=True,
        env=dict(os.environ, SDL_VIDEODRIVER="dummy", SDL_AUDIODRIVER="dummy"),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def run_ecgame_sequence(target: Path, keys: list[tuple[str, float]]) -> None:
    cmd = [
        "dosbox-x",
        "-defaultconf",
        "-nopromptfolder",
        "-defaultdir",
        str(target),
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
        "output=surface",
        "-c",
        f"mount c {target}",
        "-c",
        "c:",
        "-c",
        "ECGAME",
    ]
    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    env["TERM"] = "dumb"
    child = subprocess.Popen(
        cmd,
        env=env,
        stdin=subprocess.PIPE,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    try:
        time.sleep(5.0)
        for text, delay in keys:
            assert child.stdin is not None
            child.stdin.write(text)
            child.stdin.flush()
            time.sleep(delay)
    finally:
        if child.stdin is not None:
            child.stdin.close()
        time.sleep(2.0)
        child.kill()
        child.wait(timeout=5)


def read_planet_bytes(target: Path, record_index_1_based: int) -> bytes:
    data = (target / "PLANETS.DAT").read_bytes()
    start = (record_index_1_based - 1) * PLANET_RECORD_SIZE
    return data[start : start + PLANET_RECORD_SIZE]


def read_fleet_bytes(target: Path, record_index_1_based: int) -> bytes:
    data = (target / "FLEETS.DAT").read_bytes()
    start = (record_index_1_based - 1) * FLEET_RECORD_SIZE
    return data[start : start + FLEET_RECORD_SIZE]


def summarize_unload_case(target: Path) -> dict[str, int]:
    planet = read_planet_bytes(target, HOMEWORLD_PLANET_INDEX)
    fleet = read_fleet_bytes(target, HOMEWORLD_FLEET_INDEX)
    return {
        "planet_armies": planet[0x58],
        "fleet_transports": int.from_bytes(fleet[0x2C:0x2E], "little"),
        "fleet_loaded_armies": int.from_bytes(fleet[0x2E:0x30], "little"),
    }


def summarize_army_build_case(target: Path) -> dict[str, int]:
    planet = read_planet_bytes(target, HOMEWORLD_PLANET_INDEX)
    return {
        "planet_armies": planet[0x58],
        "build_points_slot0": planet[0x24],
        "build_kind_slot0": planet[0x2E],
    }


def summarize_battery_build_case(target: Path) -> dict[str, int]:
    planet = read_planet_bytes(target, HOMEWORLD_PLANET_INDEX)
    return {
        "ground_batteries": planet[0x5A],
        "build_points_slot0": planet[0x24],
        "build_kind_slot0": planet[0x2E],
    }


def summarize_scout_merge_case(target: Path) -> dict[str, int]:
    fleets = (target / "FLEETS.DAT").read_bytes()
    records = [fleets[i : i + FLEET_RECORD_SIZE] for i in range(0, len(fleets), FLEET_RECORD_SIZE)]
    owner1_scout_fleets = []
    for idx, record in enumerate(records, start=1):
        if len(record) < FLEET_RECORD_SIZE:
            continue
        if record[0x02] != 1:
            continue
        scouts = record[0x24]
        if scouts:
            owner1_scout_fleets.append(
                {
                    "record": idx,
                    "fleet_id": record[0x05],
                    "coords_x": record[0x0B],
                    "coords_y": record[0x0C],
                    "scouts": scouts,
                }
            )
    return {
        "owner1_scout_fleets_found": len(owner1_scout_fleets),
        "owner1_max_scouts_in_fleet": max((item["scouts"] for item in owner1_scout_fleets), default=0),
        "owner1_new_scout_fleet_record": owner1_scout_fleets[-1]["record"] if owner1_scout_fleets else 0,
    }


def setup_planet_unload_overflow_case(target: Path) -> None:
    planets = target / "PLANETS.DAT"
    fleets = target / "FLEETS.DAT"
    patch_u8(planets, planet_offset(HOMEWORLD_PLANET_INDEX, 0x58), 250)
    patch_u16(fleets, fleet_offset(HOMEWORLD_FLEET_INDEX, 0x2C), 20)
    patch_u16(fleets, fleet_offset(HOMEWORLD_FLEET_INDEX, 0x2E), 20)
    patch_u8(fleets, fleet_offset(HOMEWORLD_FLEET_INDEX, 0x24), 0)


def setup_planet_army_build_overflow_case(target: Path) -> None:
    planets = target / "PLANETS.DAT"
    patch_u8(planets, planet_offset(HOMEWORLD_PLANET_INDEX, 0x58), 255)
    patch_u8(planets, planet_offset(HOMEWORLD_PLANET_INDEX, 0x24), 100)
    patch_u8(planets, planet_offset(HOMEWORLD_PLANET_INDEX, 0x2E), 8)


def setup_planet_battery_build_overflow_case(target: Path) -> None:
    planets = target / "PLANETS.DAT"
    patch_u8(planets, planet_offset(HOMEWORLD_PLANET_INDEX, 0x5A), 255)
    patch_u8(planets, planet_offset(HOMEWORLD_PLANET_INDEX, 0x24), 100)
    patch_u8(planets, planet_offset(HOMEWORLD_PLANET_INDEX, 0x2E), 7)


def setup_scout_merge_overflow_case(target: Path) -> None:
    fleets = target / "FLEETS.DAT"
    player = target / "PLAYER.DAT"
    patch_u8(player, 0x00, 0xFF)

    for fleet_index in (1, 2):
        base = fleet_offset(fleet_index, 0)
        patch_u8(fleets, base + 0x24, 200)
        patch_u16(fleets, base + 0x26, 0)
        patch_u16(fleets, base + 0x28, 0)
        patch_u16(fleets, base + 0x2A, 0)
        patch_u16(fleets, base + 0x2C, 0)
        patch_u16(fleets, base + 0x2E, 0)
        patch_u16(fleets, base + 0x30, 0)
        patch_u8(fleets, base + 0x09, 6)
        patch_u8(fleets, base + 0x0A, 0)
        patch_u8(fleets, base + 0x1F, 14)
        patch_bytes(fleets, base + 0x20, bytes(HOMEWORLD_COORDS))

    for fleet_index in (3, 4):
        base = fleet_offset(fleet_index, 0)
        patch_u8(fleets, base + 0x24, 0)
        patch_u16(fleets, base + 0x26, 0)
        patch_u16(fleets, base + 0x28, 0)
        patch_u16(fleets, base + 0x2A, 0)
        patch_u16(fleets, base + 0x2C, 0)
        patch_u16(fleets, base + 0x2E, 0)
        patch_u16(fleets, base + 0x30, 0)


def write_summary(path: Path, title: str, before: dict[str, int], after: dict[str, int]) -> None:
    lines = [title, "", "Before:"]
    for key, value in before.items():
        lines.append(f"  {key} = {value}")
    lines.append("")
    lines.append("After:")
    for key, value in after.items():
        lines.append(f"  {key} = {value}")
    lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


def probe_unload_overflow(root: Path) -> tuple[dict[str, int], dict[str, int]]:
    target = root / "unload-overflow"
    reset_dir(target)
    setup_planet_unload_overflow_case(target)
    before = summarize_unload_case(target)
    run_ecgame_sequence(
        target,
        [
            (" ", 1.2),
            (" ", 1.2),
            (" ", 1.2),
            ("F", 1.0),
            ("U", 1.0),
            ("\r", 1.0),
            ("\r", 1.0),
            ("\r", 1.0),
            ("Q", 0.8),
            ("Q", 0.8),
            ("Y", 1.2),
        ],
    )
    after = summarize_unload_case(target)
    write_summary(target / "summary.txt", "Planet unload overflow probe", before, after)
    return before, after


def probe_army_build_overflow(root: Path) -> tuple[dict[str, int], dict[str, int]]:
    target = root / "army-build-overflow"
    reset_dir(target)
    setup_planet_army_build_overflow_case(target)
    before = summarize_army_build_case(target)
    run_ecmaint(target)
    after = summarize_army_build_case(target)
    write_summary(target / "summary.txt", "Planet army build overflow probe", before, after)
    return before, after


def probe_battery_build_overflow(root: Path) -> tuple[dict[str, int], dict[str, int]]:
    target = root / "battery-build-overflow"
    reset_dir(target)
    setup_planet_battery_build_overflow_case(target)
    before = summarize_battery_build_case(target)
    run_ecmaint(target)
    after = summarize_battery_build_case(target)
    write_summary(target / "summary.txt", "Planet battery build overflow probe", before, after)
    return before, after


def probe_scout_merge_overflow(root: Path) -> tuple[dict[str, int], dict[str, int]]:
    target = root / "scout-merge-overflow"
    reset_dir(target)
    setup_scout_merge_overflow_case(target)
    before = summarize_scout_merge_case(target)
    run_ecmaint(target)
    after = summarize_scout_merge_case(target)
    write_summary(target / "summary.txt", "Scout merge overflow probe", before, after)
    return before, after


def main() -> int:
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)

    unload_before, unload_after = probe_unload_overflow(root)
    army_before, army_after = probe_army_build_overflow(root)
    battery_before, battery_after = probe_battery_build_overflow(root)
    scout_before, scout_after = probe_scout_merge_overflow(root)

    print(f"Oracle probe artifacts written under {root}")
    print("")
    print("Planet unload overflow:")
    print(f"  before={unload_before}")
    print(f"  after ={unload_after}")
    print("")
    print("Planet army build overflow:")
    print(f"  before={army_before}")
    print(f"  after ={army_after}")
    print("")
    print("Planet battery build overflow:")
    print(f"  before={battery_before}")
    print(f"  after ={battery_after}")
    print("")
    print("Scout merge overflow:")
    print(f"  before={scout_before}")
    print(f"  after ={scout_after}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
