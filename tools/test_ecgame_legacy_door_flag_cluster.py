"""
Preserved reference for the focused remaining-field sweep on the legacy
DOOR.SYS parser window.

The resulting artifact is tracked at:
  artifacts/ecgame-startup/legacy-door-flag-cluster.json

This script intentionally mirrors the smaller ad hoc batch that tested:
  - line 07
  - line 08
  - line 09
  - line 17

against the legacy baseline and showed no change in:
  - parser sequence shape
  - stable loop-local 6 -> 17 progression
  - exit code 0x1C

It is kept as a repo placeholder to document that this focused cluster was
already covered and should not be re-run unless the harness changes.
"""

from pathlib import Path


def main() -> None:
    artifact = Path("artifacts/ecgame-startup/legacy-door-flag-cluster.json")
    print(artifact.read_text(encoding="utf-8"))


if __name__ == "__main__":
    main()
