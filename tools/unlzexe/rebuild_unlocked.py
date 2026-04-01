#!/usr/bin/env python3
from __future__ import annotations

import argparse
import struct
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
UNLZEXE_DIR = REPO_ROOT / "tools" / "unlzexe"
UNLOCKED_DIR = REPO_ROOT / "NC_UNLOCKED"

MZ_SIGNATURE = 0x5A4D
MZ_CBLP_OFFSET = 2
MZ_CP_OFFSET = 4


@dataclass(frozen=True)
class AssetSpec:
    stem: str
    source_name: str
    strategy: str


ASSET_SPECS = (
    AssetSpec("ECGAME", "ECGAMEU.EXE", "fix_size_fields"),
    AssetSpec("ECMAINT", "ECMAINT_CLEAN.EXE", "copy"),
    AssetSpec("ECUTIL", "ECUTIL_CLEAN.EXE", "copy"),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Rebuild the curated runnable executables under NC_UNLOCKED/ from "
            "the preserved tools/unlzexe artifacts."
        )
    )
    parser.add_argument(
        "--asset",
        choices=[spec.stem.lower() for spec in ASSET_SPECS],
        action="append",
        help="Rebuild only the selected asset. Defaults to all.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=UNLOCKED_DIR,
        help="Directory where the rebuilt files will be written.",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help="Verify the rebuilt output after writing it.",
    )
    return parser.parse_args()


def selected_specs(names: list[str] | None) -> list[AssetSpec]:
    if not names:
        return list(ASSET_SPECS)
    wanted = set(name.upper() for name in names)
    return [spec for spec in ASSET_SPECS if spec.stem in wanted]


def actual_mz_size_fields(size: int) -> tuple[int, int]:
    cp = (size + 511) // 512
    cblp = size % 512
    return cblp, cp


def patch_mz_size_fields(data: bytes) -> bytes:
    if len(data) < 32:
        raise ValueError("MZ image too small")
    image = bytearray(data)
    signature = struct.unpack_from("<H", image, 0)[0]
    if signature != MZ_SIGNATURE:
        raise ValueError("not an MZ executable")
    cblp, cp = actual_mz_size_fields(len(image))
    struct.pack_into("<H", image, MZ_CBLP_OFFSET, cblp)
    struct.pack_into("<H", image, MZ_CP_OFFSET, cp)
    return bytes(image)


def build_bytes(spec: AssetSpec) -> bytes:
    source_path = UNLZEXE_DIR / spec.source_name
    data = source_path.read_bytes()
    if spec.strategy == "copy":
        return data
    if spec.strategy == "fix_size_fields":
        return patch_mz_size_fields(data)
    raise ValueError(f"unknown strategy: {spec.strategy}")


def verify_output(path: Path) -> None:
    data = path.read_bytes()
    if len(data) < 32:
        raise ValueError(f"{path} is too small to be an MZ executable")
    mz = struct.unpack("<16H", data[:32])
    if mz[0] != MZ_SIGNATURE:
        raise ValueError(f"{path} is not an MZ executable")
    actual_cblp, actual_cp = actual_mz_size_fields(len(data))
    if mz[1] != actual_cblp or mz[2] != actual_cp:
        raise ValueError(
            f"{path} size fields mismatch: header cp/cblp={mz[2]}/{mz[1]}, "
            f"actual cp/cblp={actual_cp}/{actual_cblp}"
        )


def rebuild_unlocked_dir(
    output_dir: Path = UNLOCKED_DIR,
    *,
    assets: list[str] | None = None,
    verify: bool = False,
) -> list[Path]:
    output_dir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for spec in selected_specs(assets):
        out_path = output_dir / f"{spec.stem}.EXE"
        out_path.write_bytes(build_bytes(spec))
        if verify:
            verify_output(out_path)
        written.append(out_path)
    return written


def main() -> int:
    args = parse_args()
    written = rebuild_unlocked_dir(
        args.output_dir,
        assets=args.asset,
        verify=args.verify,
    )
    for path in written:
        print(path.relative_to(REPO_ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
