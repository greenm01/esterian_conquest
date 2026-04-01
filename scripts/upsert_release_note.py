#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path


START_MARKER = "<!-- NC-RUST-VERIFY:START -->"
END_MARKER = "<!-- NC-RUST-VERIFY:END -->"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Insert or replace the nc-connect verification note block in a release body."
    )
    parser.add_argument(
        "--body-file",
        type=Path,
        required=True,
        help="File containing the existing release body.",
    )
    parser.add_argument(
        "--note-file",
        type=Path,
        required=True,
        help="File containing the replacement note block.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        required=True,
        help="Path to write the merged release body.",
    )
    return parser.parse_args()


def strip_legacy_top_note(body: str) -> str:
    legacy_heading = "## Verify Rust downloads"
    if not body.startswith(legacy_heading):
        return body
    separator = "\n\nPublic Nostrian Conquest release artifacts."
    split_at = body.find(separator)
    if split_at == -1:
        return body
    return body[split_at + 2 :].lstrip()


def merge_body(body: str, note: str) -> str:
    body = strip_legacy_top_note(body).strip()
    note = note.strip()
    if START_MARKER in body and END_MARKER in body:
        start = body.index(START_MARKER)
        end = body.index(END_MARKER) + len(END_MARKER)
        merged = f"{body[:start].rstrip()}\n\n{note}\n\n{body[end:].lstrip()}".strip()
    elif body:
        merged = f"{note}\n\n{body}"
    else:
        merged = note
    return merged.strip() + "\n"


def main() -> None:
    args = parse_args()
    body = args.body_file.read_text(encoding="utf-8")
    note = args.note_file.read_text(encoding="utf-8")
    args.output.write_text(merge_body(body, note), encoding="utf-8")


if __name__ == "__main__":
    main()
