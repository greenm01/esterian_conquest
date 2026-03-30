#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Write a deterministic SHA256SUMS.txt manifest for release assets."
    )
    parser.add_argument(
        "--output",
        type=Path,
        required=True,
        help="Path to the checksum manifest to write.",
    )
    parser.add_argument(
        "assets",
        nargs="+",
        type=Path,
        help="Release asset files to include in the manifest.",
    )
    return parser.parse_args()


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while True:
            chunk = handle.read(1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    args = parse_args()
    rows: list[tuple[str, str]] = []
    for asset in args.assets:
        if not asset.is_file():
            raise SystemExit(f"asset not found: {asset}")
        rows.append((asset.name, file_sha256(asset)))
    rows.sort(key=lambda row: row[0])
    manifest = "".join(f"{digest}  {name}\n" for name, digest in rows)
    args.output.write_text(manifest, encoding="utf-8")


if __name__ == "__main__":
    main()
