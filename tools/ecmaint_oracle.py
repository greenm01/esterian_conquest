#!/usr/bin/env python3
"""Prepare and run black-box ECMAINT oracle experiments."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_BASELINE = ROOT / "fixtures" / "ecmaint-post" / "v1.5"
CORE_FILES = [
    "PLAYER.DAT",
    "PLANETS.DAT",
    "FLEETS.DAT",
    "BASES.DAT",
    "IPBM.DAT",
    "SETUP.DAT",
    "CONQUEST.DAT",
]
REPORT_FILES = ["MESSAGES.DAT", "RESULTS.DAT", "RANKINGS.TXT", "ERRORS.TXT"]


@dataclass(frozen=True)
class FileDiff:
    name: str
    before_size: int
    after_size: int
    differing_offsets: list[int]

    @property
    def differing_bytes(self) -> int:
        return len(self.differing_offsets)


def repo_relative(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def copy_tree(source: Path, target: Path) -> None:
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(source, target)


def snapshot_dir(target: Path, snapshot_name: str) -> Path:
    snapshot_root = target / ".oracle"
    snapshot_path = snapshot_root / snapshot_name
    if snapshot_path.exists():
        shutil.rmtree(snapshot_path)
    snapshot_path.mkdir(parents=True, exist_ok=True)
    for name in CORE_FILES + REPORT_FILES:
        source = target / name
        if source.exists():
            shutil.copy2(source, snapshot_path / name)
    return snapshot_path


def read_bytes(path: Path) -> bytes:
    return path.read_bytes() if path.exists() else b""


def diff_offsets(before: bytes, after: bytes) -> list[int]:
    shared = min(len(before), len(after))
    offsets = [idx for idx in range(shared) if before[idx] != after[idx]]
    offsets.extend(range(shared, max(len(before), len(after))))
    return offsets


def summarize_clusters(offsets: list[int]) -> str:
    if not offsets:
        return "[]"
    clusters: list[str] = []
    start = prev = offsets[0]
    for offset in offsets[1:]:
        if offset == prev + 1:
            prev = offset
            continue
        clusters.append(f"{start}" if start == prev else f"{start}-{prev}")
        start = prev = offset
    clusters.append(f"{start}" if start == prev else f"{start}-{prev}")
    return "[" + ", ".join(clusters) + "]"


def collect_diffs(before_dir: Path, after_dir: Path) -> list[FileDiff]:
    diffs: list[FileDiff] = []
    for name in CORE_FILES + REPORT_FILES:
        before = read_bytes(before_dir / name)
        after = read_bytes(after_dir / name)
        diffs.append(
            FileDiff(
                name=name,
                before_size=len(before),
                after_size=len(after),
                differing_offsets=diff_offsets(before, after),
            )
        )
    return diffs


def run_ecmaint(target: Path, extra_env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env.setdefault("SDL_VIDEODRIVER", "dummy")
    env.setdefault("SDL_AUDIODRIVER", "dummy")
    if extra_env:
        env.update(extra_env)

    cmd = [
        "dosbox-x",
        "-defaultconf",
        "-nopromptfolder",
        "-nogui",
        "-nomenu",
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
        "ECMAINT /R",
        "-c",
        "exit",
    ]
    return subprocess.run(cmd, env=env, text=True, capture_output=True)


def cmd_prepare(args: argparse.Namespace) -> int:
    source = Path(args.source).resolve()
    target = Path(args.target).resolve()
    copy_tree(source, target)
    snapshot_path = snapshot_dir(target, "prepared")
    print("Prepared ECMAINT oracle directory")
    print(f"  source={repo_relative(source)}")
    print(f"  target={target}")
    print(f"  snapshot={snapshot_path}")
    print("  next step: submit player orders or mutate files, then run:")
    print(f"    python3 tools/ecmaint_oracle.py run {target}")
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    target = Path(args.target).resolve()
    if not target.exists():
        raise SystemExit(f"target does not exist: {target}")

    before_snapshot = snapshot_dir(target, "before-ecmaint")
    result = run_ecmaint(target)
    after_snapshot = snapshot_dir(target, "after-ecmaint")
    diffs = collect_diffs(before_snapshot, target)

    oracle_root = target / ".oracle"
    (oracle_root / "dosbox.stdout.txt").write_text(result.stdout, encoding="utf-8", errors="ignore")
    (oracle_root / "dosbox.stderr.txt").write_text(result.stderr, encoding="utf-8", errors="ignore")

    print("ECMAINT oracle run complete")
    print(f"  target={target}")
    print(f"  before_snapshot={before_snapshot}")
    print(f"  after_snapshot={after_snapshot}")
    print(f"  dosbox_exit_code={result.returncode}")
    for diff in diffs:
        print(
            f"  {diff.name}: size {diff.before_size} -> {diff.after_size}, "
            f"differing_bytes={diff.differing_bytes}, clusters={summarize_clusters(diff.differing_offsets)}"
        )
    if (target / "ERRORS.TXT").exists():
        first_line = (target / "ERRORS.TXT").read_text(errors="ignore").splitlines()[:1]
        if first_line:
            print(f"  ERRORS.TXT first line: {first_line[0]}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    prepare = subparsers.add_parser("prepare", help="copy a baseline into a working directory")
    prepare.add_argument("target", help="working directory to create")
    prepare.add_argument(
        "source",
        nargs="?",
        default=str(DEFAULT_BASELINE),
        help="baseline directory to copy (default: fixtures/ecmaint-post/v1.5)",
    )
    prepare.set_defaults(func=cmd_prepare)

    run = subparsers.add_parser("run", help="snapshot, run ECMAINT, and diff results")
    run.add_argument("target", help="working directory to process in place")
    run.set_defaults(func=cmd_run)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
