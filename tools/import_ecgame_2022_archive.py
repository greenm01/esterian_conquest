#!/usr/bin/env python3
"""Import preserved ECGAME client assets from the 2022 nested zip archive."""

from __future__ import annotations

import io
import sys
import zipfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
OUTPUT_ROOT = REPO_ROOT / "artifacts" / "ecgame-client" / "archive-2022"

LOGS_ZIP = "ec-logs-2022.zip"
SELECTED_LOGS = [
    "ec/2022-07-23-NEW-GAME.txt",
]
SELECTED_ANSI = [
    "ec/ansi/first-time-menu.ans",
    "ec/ansi/ftj-join.ans",
    "ec/ansi/post-join-first-menu.ans",
    "ec/ansi/ftm-view-game-intro.ans",
    "ec/ansi/ftm-help.ans",
    "ec/ansi/ftm-list.ans",
]


def extract_member(zf: zipfile.ZipFile, member: str, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_bytes(zf.read(member))


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: import_ecgame_2022_archive.py <outer_zip>", file=sys.stderr)
        return 2

    outer_zip_path = Path(argv[1])
    with zipfile.ZipFile(outer_zip_path) as outer_zip:
        logs_data = outer_zip.read(LOGS_ZIP)

    with zipfile.ZipFile(io.BytesIO(logs_data)) as logs_zip:
        for member in SELECTED_LOGS:
            extract_member(logs_zip, member, OUTPUT_ROOT / Path(member).name)
        for member in SELECTED_ANSI:
            extract_member(logs_zip, member, OUTPUT_ROOT / "ansi" / Path(member).name)

    manifest = [
        "ECGAME 2022 client archive import",
        "================================",
        "",
        "Source outer zip: imported from a local archive outside the repo",
        f"Nested zip used: {LOGS_ZIP}",
        "",
        "Imported files:",
    ]
    manifest.extend(f"- {Path(member).name}" for member in SELECTED_LOGS)
    manifest.extend(f"- ansi/{Path(member).name}" for member in SELECTED_ANSI)
    manifest.append("")
    manifest.append("Notes:")
    manifest.append("- .ans files are preserved ANSI screen assets from the 2022 log bundle.")
    manifest.append("- 2022-07-23-NEW-GAME.txt contains a richer escape-sequence transcript")
    manifest.append("  of the intro/new-game flow than the earlier plain text extracts.")
    manifest.append("")
    (OUTPUT_ROOT / "README.txt").write_text("\n".join(manifest), encoding="utf-8")

    print(OUTPUT_ROOT)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
