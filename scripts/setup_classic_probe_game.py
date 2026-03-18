#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_DIR = REPO_ROOT / "rust"
RUN_CLASSIC = REPO_ROOT / "tools" / "run_ecgame.sh"

PLAYER_SPECS = [
    (1, "SYSOP", "Auroran Combine", 42),
    (2, "HECATE", "Red Horizon Pact", 50),
    (3, "ORION", "Vela Syndicate", 36),
    (4, "TESS", "Helios Crown", 58),
]

PLANET_SPECS = [
    (1, 1, "Foundation", 100, 0, 10, 4),
    (2, 2, "Red Haven", 100, 0, 10, 4),
    (3, 3, "Vela Prime", 100, 0, 10, 4),
    (4, 4, "Crownfall", 100, 0, 10, 4),
    (5, 4, "Helios Prime", 136, 30, 10, 6),
    (8, 3, "Outer Vela", 128, 26, 8, 5),
    (12, 3, "Vela Gate", 104, 18, 7, 4),
    (13, 2, "Red Bastion", 132, 28, 12, 7),
    (15, 2, "Crucible", 118, 24, 14, 8),
    (16, 1, "Aurora Prime", 144, 48, 12, 6),
    (17, 1, "Relay", 96, 20, 5, 3),
    (19, 1, "Outrider", 84, 14, 3, 2),
]

REPORT_FAMILY_LABELS = [
    ("Fleet battle report", "fleet-battle"),
    ("Bombardment mission report", "bombard"),
    ("Invasion mission report", "invade"),
    ("Blitz mission report", "blitz"),
    ("Viewing mission report", "view"),
    ("Scouting mission report", "scout"),
    ("Guard/Blockade World mission report", "guard-blockade"),
    ("Salvage mission report", "salvage"),
    ("Move mission report", "move"),
    ("Fleet encounter report", "fleet-encounter"),
]

PLAYER_RECORD_SIZE = 110
PLAYER_MESSAGES_PENDING_OFFSET = 0x30
PLAYER_MESSAGES_PENDING_HI_OFFSET = 0x31
PLAYER_MESSAGES_REVIEW_CARRY_OFFSET = 0x32
PLAYER_MESSAGES_REVIEW_CARRY_HI_OFFSET = 0x33
PLAYER_REPORTS_PENDING_OFFSET = 0x34
PLAYER_REPORTS_PENDING_HI_OFFSET = 0x35
PLAYER_REPORTS_REVIEW_CARRY_OFFSET = 0x36
PLAYER_REPORTS_REVIEW_CARRY_HI_OFFSET = 0x37
PLAYER_RESULTS_CHAIN_FLAG_OFFSET = 0x38
PLAYER_RESULTS_CHAIN_NEXT_FREE_OFFSET = 0x3C
PLAYER_LAST_RUN_YEAR_OFFSET = 0x4E
CLASSIC_RECORD_SIZE = 84
CLASSIC_RESULTS_TEXT_SIZE = 72
CLASSIC_MESSAGES_TEXT_SIZE = 75
END_OF_TRANSMISSION = "<end of transmission>"


def cargo_profile_dir(release: bool) -> str:
    return "release" if release else "debug"


def build_workspace_binary(package: str, release: bool) -> Path:
    cmd = ["cargo", "build", "-q", "-p", package]
    if release:
        cmd.insert(2, "--release")
    subprocess.run(cmd, cwd=RUST_DIR, check=True)
    return RUST_DIR / "target" / cargo_profile_dir(release) / package


def run_ec_cli(cli_binary: Path, *args: str) -> str:
    result = subprocess.run(
        [str(cli_binary), *args],
        cwd=RUST_DIR,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def set_player_specs(cli_binary: Path, target: Path, player_one_alias: str, player_one_empire: str) -> None:
    for record, handle, empire, tax_rate in PLAYER_SPECS:
        if record == 1:
            handle = player_one_alias
            empire = player_one_empire
        if record == 1:
            run_ec_cli(
                cli_binary,
                "player-join",
                str(target),
                str(record),
                handle,
                empire,
                "Foundation",
            )
        else:
            run_ec_cli(cli_binary, "player-name", str(target), str(record), handle, empire)
        run_ec_cli(cli_binary, "player-tax", str(target), str(record), str(tax_rate))


def set_planet_specs(cli_binary: Path, target: Path) -> None:
    for record, owner, name, potential, stored, armies, batteries in PLANET_SPECS:
        run_ec_cli(cli_binary, "planet-owner", str(target), str(record), str(owner))
        run_ec_cli(cli_binary, "planet-name", str(target), str(record), name)
        run_ec_cli(cli_binary, "planet-potential", str(target), str(record), str(potential), "0")
        run_ec_cli(cli_binary, "planet-stored", str(target), str(record), str(stored))
        run_ec_cli(cli_binary, "planet-stats", str(target), str(record), str(armies), str(batteries))

    # Keep a little visible stardock inventory on player one's worlds for classic inspection.
    run_ec_cli(cli_binary, "planet-stardock", str(target), "16", "0", "4", "1")
    run_ec_cli(cli_binary, "planet-stardock", str(target), "16", "1", "1", "2")
    run_ec_cli(cli_binary, "planet-stardock", str(target), "17", "0", "2", "3")


def write_diplomacy_sidecar(target: Path) -> None:
    lines = [
        'relation from=1 to=2 status="enemy"',
        'relation from=2 to=1 status="enemy"',
        'relation from=1 to=3 status="enemy"',
        'relation from=3 to=1 status="enemy"',
        'relation from=1 to=4 status="enemy"',
        'relation from=4 to=1 status="enemy"',
    ]
    (target / "diplomacy.kdl").write_text("\n".join(lines) + "\n", encoding="utf-8")


def configure_enemy_fleets(cli_binary: Path, target: Path) -> None:
    run_ec_cli(cli_binary, "fleet-ships", str(target), "5", "0", "2", "8", "4", "0", "0", "0")
    run_ec_cli(cli_binary, "fleet-order", str(target), "5", "3", "5", "9", "2")

    run_ec_cli(cli_binary, "fleet-ships", str(target), "6", "0", "2", "6", "5", "0", "0", "0")
    run_ec_cli(cli_binary, "fleet-order", str(target), "6", "3", "5", "9", "2")

    run_ec_cli(cli_binary, "fleet-ships", str(target), "9", "1", "1", "4", "2", "0", "0", "0")
    run_ec_cli(cli_binary, "fleet-order", str(target), "9", "3", "5", "9", "2")

    run_ec_cli(cli_binary, "fleet-ships", str(target), "10", "0", "1", "4", "2", "0", "0", "0")
    run_ec_cli(cli_binary, "fleet-order", str(target), "10", "3", "5", "13", "5")

    run_ec_cli(cli_binary, "fleet-ships", str(target), "13", "0", "1", "4", "2", "0", "0", "0")
    run_ec_cli(cli_binary, "fleet-order", str(target), "13", "3", "5", "5", "13")

    run_ec_cli(cli_binary, "fleet-ships", str(target), "14", "0", "1", "4", "2", "0", "0", "0")
    run_ec_cli(cli_binary, "fleet-order", str(target), "14", "3", "5", "13", "13")


def configure_player_one_fleets(cli_binary: Path, target: Path) -> dict[str, int]:
    fleet_records = {
        "bombard": 1,
        "scout_system": 2,
        "move": 3,
        "salvage": 4,
    }

    run_ec_cli(cli_binary, "fleet-ships", str(target), "1", "0", "2", "4", "6", "0", "0", "0")
    run_ec_cli(cli_binary, "fleet-order", str(target), "1", "3", "6", "9", "2")

    run_ec_cli(cli_binary, "fleet-ships", str(target), "2", "1", "0", "1", "2", "0", "0", "0")
    run_ec_cli(cli_binary, "fleet-order", str(target), "2", "3", "11", "9", "2")

    run_ec_cli(cli_binary, "fleet-ships", str(target), "3", "1", "0", "0", "1", "0", "0", "0")
    run_ec_cli(cli_binary, "fleet-order", str(target), "3", "3", "1", "9", "4")

    run_ec_cli(cli_binary, "fleet-ships", str(target), "4", "0", "1", "1", "1", "0", "0", "0")
    run_ec_cli(cli_binary, "fleet-order", str(target), "4", "3", "15", "5", "2")

    return fleet_records


def summarize_reports(target: Path) -> list[str]:
    combined = b""
    for name in ("RESULTS.DAT",):
        path = target / name
        if path.exists():
            combined += path.read_bytes()
    text = combined.decode("latin-1", errors="ignore").replace("\x00", " ")
    found = [label for needle, label in REPORT_FAMILY_LABELS if needle in text]
    return found


def routed_message_entries(messages_bytes: bytes) -> list[tuple[int, bytes, str]]:
    entries: list[tuple[int, bytes, str]] = []
    current_kind: int | None = None
    current_tail_suffix: bytes | None = None
    current_chunks: list[str] = []

    for offset in range(0, len(messages_bytes), CLASSIC_RECORD_SIZE):
        record = messages_bytes[offset : offset + CLASSIC_RECORD_SIZE]
        if len(record) != CLASSIC_RECORD_SIZE:
            continue
        text = record[1 : 1 + CLASSIC_MESSAGES_TEXT_SIZE].split(b"\x00", 1)[0].decode(
            "cp437",
            errors="replace",
        )
        if not text:
            continue
        if text.startswith("For Empire #"):
            if current_kind is not None and current_tail_suffix is not None:
                entries.append((current_kind, current_tail_suffix, "".join(current_chunks)))
            current_kind = record[0]
            current_tail_suffix = bytes(record[76:84])
            current_chunks = [text]
        elif current_kind is not None:
            current_chunks.append(text)

    if current_kind is not None and current_tail_suffix is not None:
        entries.append((current_kind, current_tail_suffix, "".join(current_chunks)))

    return entries


def build_classic_results_records(
    routed_entries: list[tuple[int, bytes, str]],
    prefix: str,
) -> bytes:
    filtered: list[tuple[int, bytes, str]] = []
    for kind, tail_suffix, text in routed_entries:
        if not text.startswith(prefix):
            continue
        full_tail = bytearray(10)
        full_tail[2:] = tail_suffix
        filtered.append((kind, bytes(full_tail), text[len(prefix) :]))

    if not filtered:
        return b""

    record_counts = [
        (len(text.encode("cp437", errors="replace")) + CLASSIC_RESULTS_TEXT_SIZE - 1)
        // CLASSIC_RESULTS_TEXT_SIZE
        + 1
        for _, _, text in filtered
    ]
    header_record_indexes: list[int] = []
    next_header_record_index = 0
    for record_count in record_counts:
        header_record_indexes.append(next_header_record_index)
        next_header_record_index += record_count

    output = bytearray()
    for idx, (kind, tail_template, text) in enumerate(filtered):
        chain_id = 0 if idx == 0 else header_record_indexes[idx - 1] + 1
        next_chain_id = (
            header_record_indexes[idx + 1] + 1 if idx + 1 < len(header_record_indexes) else 0
        )
        header_tail = bytearray(tail_template)
        header_tail[0:2] = chain_id.to_bytes(2, "little")
        header_tail[2:4] = b"\x00\x00"
        header_tail[4:6] = next_chain_id.to_bytes(2, "little")
        header_tail[6:8] = b"\x00\x00"

        continuation_tail = bytearray(tail_template)
        continuation_tail[0:2] = chain_id.to_bytes(2, "little")
        continuation_tail[2:4] = b"\x00\x00"
        continuation_tail[4:8] = b"\x00\x00\x00\x00"

        payload = text.encode("cp437", errors="replace")
        for chunk_idx in range(0, len(payload), CLASSIC_RESULTS_TEXT_SIZE):
            chunk = payload[chunk_idx : chunk_idx + CLASSIC_RESULTS_TEXT_SIZE]
            record = bytearray(CLASSIC_RECORD_SIZE)
            record[0] = kind
            record[1] = len(chunk)
            record[2 : 2 + len(chunk)] = chunk
            record[74:84] = header_tail if chunk_idx == 0 else continuation_tail
            output.extend(record)

        eot = END_OF_TRANSMISSION.encode("cp437")
        record = bytearray(CLASSIC_RECORD_SIZE)
        record[0] = kind
        record[1] = len(eot)
        record[2 : 2 + len(eot)] = eot
        record[74:84] = continuation_tail
        output.extend(record)

    return bytes(output)


def prepare_player_one_classic_report_probe(
    target: Path,
    *,
    player_record: int,
    empire_name: str,
) -> None:
    messages_bytes = (target / "MESSAGES.DAT").read_bytes()
    routed_entries = routed_message_entries(messages_bytes)
    routed_prefix = f'For Empire #{player_record} "{empire_name}": '
    results_bytes = build_classic_results_records(routed_entries, routed_prefix)
    (target / "RESULTS.DAT").write_bytes(results_bytes)
    (target / "MESSAGES.DAT").write_bytes(b"")

    conquest_bytes = (target / "CONQUEST.DAT").read_bytes()
    current_year = int.from_bytes(conquest_bytes[0:2], "little")

    player_path = target / "PLAYER.DAT"
    player_bytes = bytearray(player_path.read_bytes())
    player_count = len(player_bytes) // PLAYER_RECORD_SIZE
    for index in range(player_count):
        base = index * PLAYER_RECORD_SIZE
        player_bytes[base + PLAYER_MESSAGES_PENDING_OFFSET] = 0
        player_bytes[base + PLAYER_MESSAGES_PENDING_HI_OFFSET] = 0
        player_bytes[base + PLAYER_MESSAGES_REVIEW_CARRY_OFFSET] = 0
        player_bytes[base + PLAYER_MESSAGES_REVIEW_CARRY_HI_OFFSET] = 0
        player_bytes[base + PLAYER_REPORTS_PENDING_OFFSET] = 0
        player_bytes[base + PLAYER_REPORTS_PENDING_HI_OFFSET] = 0
        player_bytes[base + PLAYER_REPORTS_REVIEW_CARRY_OFFSET] = 0
        player_bytes[base + PLAYER_REPORTS_REVIEW_CARRY_HI_OFFSET] = 0
        player_bytes[base + PLAYER_RESULTS_CHAIN_FLAG_OFFSET : base + PLAYER_RESULTS_CHAIN_FLAG_OFFSET + 2] = b"\x00\x00"
        player_bytes[
            base + PLAYER_RESULTS_CHAIN_NEXT_FREE_OFFSET : base + PLAYER_RESULTS_CHAIN_NEXT_FREE_OFFSET + 2
        ] = b"\x00\x00"
    if player_count >= player_record:
        base = (player_record - 1) * PLAYER_RECORD_SIZE
        header_record_indexes = [
            idx
            for idx in range(0, len(results_bytes), CLASSIC_RECORD_SIZE)
            if results_bytes[idx + 2 : idx + 7].decode("cp437", errors="replace") == "From "
        ]
        if header_record_indexes:
            player_bytes[
                base + PLAYER_RESULTS_CHAIN_FLAG_OFFSET : base + PLAYER_RESULTS_CHAIN_FLAG_OFFSET + 2
            ] = (1).to_bytes(2, "little")
            player_bytes[
                base + PLAYER_RESULTS_CHAIN_NEXT_FREE_OFFSET : base + PLAYER_RESULTS_CHAIN_NEXT_FREE_OFFSET + 2
            ] = ((header_record_indexes[-1] // CLASSIC_RECORD_SIZE) + 1).to_bytes(2, "little")
        # The maint export now seeds the later classic results-chain state at
        # PLAYER[0x38..0x3f]. Preserve that region here and only clear the old
        # message/review-family bytes so ECGAME enters the undeleted-reports
        # path without inventing blank messages.
        # Classic login review only makes sense when the player has not yet
        # seen the current game year. Fresh maint output should therefore look
        # like "results from last year are waiting", not "already logged in
        # this same year".
        if results_bytes and current_year > 0:
            previous_year = current_year - 1
            player_bytes[
                PLAYER_LAST_RUN_YEAR_OFFSET : PLAYER_LAST_RUN_YEAR_OFFSET + 2
            ] = previous_year.to_bytes(2, "little")
    player_path.write_bytes(player_bytes)


def print_summary(target: Path, turns: int, alias: str, empire: str, fleet_records: dict[str, int]) -> None:
    report_labels = summarize_reports(target)
    print()
    print(f"Prepared classic probe game at {target}")
    print("Players: 4")
    print(f"Turns simulated with rust maint: {turns}")
    print(f"Classic caller alias for player 1: {alias}")
    print(f"Player 1 empire: {empire}")
    print("Configured player 1 fleets:")
    for label, record in fleet_records.items():
        print(f"  - {label}: fleet record {record}")
    print("Detected report families:")
    for label in report_labels:
        print(f"  - {label}")
    if not report_labels:
        print("  - none detected")
    print("Classic launch command:")
    print(f"  {RUN_CLASSIC} {target} 1 {alias}")


def main() -> None:
    parser = argparse.ArgumentParser(
        description=(
            "Create a fresh 4-player Rust-backed campaign, seed it with busy player-1 fleets, "
            "run several Rust maint turns, and launch classic ECGAME in DOSBox-X."
        )
    )
    parser.add_argument("target_dir", help="Directory to create or replace.")
    parser.add_argument(
        "--seed",
        type=int,
        default=1515,
        help="World-generation seed. Default: 1515.",
    )
    parser.add_argument(
        "--turns",
        type=int,
        default=4,
        help="Rust maint turns to run before launch. Default: 4.",
    )
    parser.add_argument(
        "--alias",
        default="SYSOP",
        help="Classic caller alias for player 1. Default: SYSOP.",
    )
    parser.add_argument(
        "--empire",
        default="Auroran Combine",
        help="Empire name for player 1. Default: Auroran Combine.",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Remove the target directory first if it already exists.",
    )
    parser.add_argument(
        "--release",
        action="store_true",
        help="Use release builds for Rust binaries.",
    )
    parser.add_argument(
        "--no-launch",
        action="store_true",
        help="Prepare the campaign but do not launch classic ECGAME.",
    )
    args = parser.parse_args()

    target = Path(args.target_dir).resolve()
    if target.exists():
        if not args.force:
            raise SystemExit(f"target already exists: {target} (use --force)")
        shutil.rmtree(target)

    cli_binary = build_workspace_binary("ec-cli", args.release)

    run_ec_cli(
        cli_binary,
        "sysop",
        "new-game",
        str(target),
        "--players",
        "4",
        "--seed",
        str(args.seed),
    )
    set_player_specs(cli_binary, target, args.alias, args.empire)
    set_planet_specs(cli_binary, target)
    configure_enemy_fleets(cli_binary, target)
    fleet_records = configure_player_one_fleets(cli_binary, target)
    write_diplomacy_sidecar(target)
    run_ec_cli(cli_binary, "maint-rust", str(target), str(args.turns))
    prepare_player_one_classic_report_probe(
        target,
        player_record=1,
        empire_name=args.empire,
    )
    run_ec_cli(cli_binary, "db-import", str(target))

    print_summary(target, args.turns, args.alias, args.empire, fleet_records)

    if args.no_launch:
        return

    env = os.environ.copy()
    subprocess.run(
        [str(RUN_CLASSIC), str(target), "1", args.alias],
        cwd=REPO_ROOT,
        check=True,
        env=env,
    )


if __name__ == "__main__":
    main()
