#!/usr/bin/env python3
"""Extract a preserved ECGAME intro-to-main-menu reference sequence.

This is not a raw ANSI byte-stream capture. It is a reproducible text
reference built from the preserved historical captures already in the repo:

- original/v1.5/EC-Screenshots-v1.11/2022-05-01-credits.txt
- original/v1.5/EC-Screenshots-v1.11/2022-05-01-intro.txt
- original/v1.5/ec-logs-2012/ec11.txt

The goal is to give the Rust client work one stable artifact for the original
opening flow:

- splash / logo
- welcome-back prompts
- undeleted reports flow
- homeworld naming
- arrival at the main menu
"""

from __future__ import annotations

from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
OUTPUT = REPO_ROOT / "artifacts" / "ecgame-client" / "intro-sequence.txt"
LOG_CAPTURE = REPO_ROOT / "original" / "v1.5" / "ec-logs-2012" / "ec11.txt"
INTRO = REPO_ROOT / "original" / "v1.5" / "EC-Screenshots-v1.11" / "2022-05-01-intro.txt"
CREDITS = REPO_ROOT / "original" / "v1.5" / "EC-Screenshots-v1.11" / "2022-05-01-credits.txt"


def extract_ec11_sequence() -> str:
    text = LOG_CAPTURE.read_text(encoding="utf-8", errors="replace")
    start_marker = "**************"
    end_marker = "MAIN COMMAND <-H,Q,X,V,A,G,P,F,T,I,B,D->"

    start = text.find(start_marker)
    if start == -1:
        raise RuntimeError(f"could not find start marker {start_marker!r} in {LOG_CAPTURE}")

    end = text.find(end_marker, start)
    if end == -1:
        raise RuntimeError(f"could not find end marker {end_marker!r} in {LOG_CAPTURE}")

    # Include the line containing the end marker.
    end = text.find("\n", end)
    if end == -1:
        end = len(text)

    return text[start:end].strip()


def main() -> int:
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)

    parts = [
        "ECGAME intro-to-main-menu reference",
        "===================================",
        "",
        "This artifact is built from preserved historical text captures.",
        "It is a text-flow reference for the Rust player client, not a raw ANSI byte stream.",
        "",
        f"Sources:",
        f"- {CREDITS.relative_to(REPO_ROOT)}",
        f"- {INTRO.relative_to(REPO_ROOT)}",
        f"- {LOG_CAPTURE.relative_to(REPO_ROOT)}",
        "",
        "CREDITS TEXT",
        "------------",
        CREDITS.read_text(encoding="utf-8", errors="replace").strip(),
        "",
        "INTRO TEXT",
        "----------",
        INTRO.read_text(encoding="utf-8", errors="replace").strip(),
        "",
        "INTRO FLOW THROUGH MAIN MENU",
        "----------------------------",
        extract_ec11_sequence(),
        "",
    ]

    OUTPUT.write_text("\n".join(parts), encoding="utf-8")
    print(OUTPUT)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
