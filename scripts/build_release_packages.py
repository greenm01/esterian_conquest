#!/usr/bin/env python3
from __future__ import annotations

import argparse
import shutil
import sys
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT))

from tools.ecgame_dropfiles import write_chain_txt
from tools.unlzexe.rebuild_unlocked import rebuild_unlocked_dir


FIXTURE_GAME_DIR = REPO_ROOT / "fixtures" / "ecutil-init" / "v1.5"
ORIGINAL_DIR = REPO_ROOT / "original" / "v1.5"
UNLOCKED_DIR = REPO_ROOT / "EC_UNLOCKED"
RELEASES_DIR = REPO_ROOT / "releases"
ZIP_TIMESTAMP = (2026, 1, 1, 0, 0, 0)
DOC_NAMES = (
    "ECREADME.DOC",
    "ECPLAYER.DOC",
    "ECQSTART.DOC",
    "WHATSNEW.DOC",
    "ECREG.DOC",
)
GAME_STATIC_NAMES = ("GALAXY.MAP",)
GAME_DAT_NAMES = (
    "BASES.DAT",
    "CONQUEST.DAT",
    "DATABASE.DAT",
    "FLEETS.DAT",
    "IPBM.DAT",
    "MESSAGES.DAT",
    "PLANETS.DAT",
    "PLAYER.DAT",
    "RESULTS.DAT",
    "SETUP.DAT",
)


@dataclass(frozen=True)
class PackageSpec:
    slug: str
    exe_root: Path
    archive_name: str
    notes_title: str
    notes_summary: str
    emulator_notes: str
    bundle_summary: str
    dosbox_status: str
    dosbox_notes: str
    dosemu_status: str
    dosemu_notes: str


PACKAGE_SPECS = (
    PackageSpec(
        slug="classic",
        exe_root=ORIGINAL_DIR,
        archive_name="ec-v1.5-classic-demo.zip",
        notes_title="Classic Packed Oracle Bundle",
        notes_summary=(
            "This package uses the original packed DOS binaries under their "
            "shipped filenames."
        ),
        emulator_notes=(
            "Use this bundle when you want the acceptance-oracle binaries. "
            "The practical target for this package is DOSBox-X."
        ),
        bundle_summary="Original packed DOS executables",
        dosbox_status="Verified",
        dosbox_notes=(
            "8s smoke pass from `/tmp` with a real game dir and known-good "
            "local-console `CHAIN.TXT`."
        ),
        dosemu_status="Not verified here",
        dosemu_notes="Packed/oracle bundle; dosemu2 is not the primary target.",
    ),
    PackageSpec(
        slug="unlocked",
        exe_root=UNLOCKED_DIR,
        archive_name="ec-v1.5-unlocked-demo.zip",
        notes_title="Unlocked Plain-MZ Bundle",
        notes_summary=(
            "This package swaps in the curated runnable plain-MZ executables "
            "from EC_UNLOCKED/ while keeping the same filenames and game layout."
        ),
        emulator_notes=(
            "Use this bundle when you want the stub-free binaries in the "
            "same runnable game layout as the classic bundle. The current "
            "ECGAME.EXE is rebuilt from the memdump-extracted unlock artifact "
            "with corrected MZ size fields so DOS loads the full image."
        ),
        bundle_summary="Curated runnable plain-MZ executables from `EC_UNLOCKED/`",
        dosbox_status="Verified",
        dosbox_notes=(
            "8s smoke pass from `/tmp`; current `ECGAME.EXE` is rebuilt from "
            "`ECGAMEU.EXE` with corrected MZ size fields."
        ),
        dosemu_status="Not verified here",
        dosemu_notes=(
            "Intended target once dosemu2 bootstrap/config issues are resolved; "
            "do not claim support yet."
        ),
    ),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Build demo-ready classic and unlocked Esterian Conquest release "
            "zip packages."
        )
    )
    parser.add_argument(
        "--variant",
        choices=[spec.slug for spec in PACKAGE_SPECS],
        action="append",
        help="Build only the selected variant. Defaults to both.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=RELEASES_DIR,
        help="Directory where the zip archives will be written.",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help="Validate the generated archives after building them.",
    )
    return parser.parse_args()


def selected_specs(variants: list[str] | None) -> list[PackageSpec]:
    if not variants:
        return list(PACKAGE_SPECS)
    wanted = set(variants)
    return [spec for spec in PACKAGE_SPECS if spec.slug in wanted]


def refresh_unlocked_bundle(specs: list[PackageSpec]) -> None:
    if any(spec.slug == "unlocked" for spec in specs):
        rebuild_unlocked_dir(UNLOCKED_DIR, verify=True)


def copy_tree_contents(src: Path, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    for path in sorted(src.iterdir(), key=lambda p: p.name):
        if path.is_dir():
            shutil.copytree(path, dest / path.name)
        else:
            shutil.copy2(path, dest / path.name)


def copy_named_files(src_root: Path, dest_root: Path, names: tuple[str, ...]) -> None:
    dest_root.mkdir(parents=True, exist_ok=True)
    for name in names:
        shutil.copy2(src_root / name, dest_root / name)


def markdown_table(headers: tuple[str, ...], rows: list[tuple[str, ...]]) -> str:
    lines = [
        "| " + " | ".join(headers) + " |",
        "|" + "|".join("---" for _ in headers) + "|",
    ]
    lines.extend("| " + " | ".join(row) + " |" for row in rows)
    return "\n".join(lines)


def package_readme(spec: PackageSpec, package_root: Path) -> str:
    status_table = markdown_table(
        ("Emulator", "Status", "Notes"),
        [
            ("DOSBox-X", spec.dosbox_status, spec.dosbox_notes),
            ("dosemu2", spec.dosemu_status, spec.dosemu_notes),
        ],
    )
    lines = [
        f"# {spec.notes_title}",
        "",
        spec.notes_summary,
        "",
        "The package contents are:",
        "",
        "- `game/`: a minimal real game directory built from a preserved joinable baseline",
        "- `dropfiles/CHAIN.TXT`: the same known-good local-console WWIV dropfile used in `game/`",
        "- `docs/`: the original bundled `.DOC` manuals",
        "",
        "## Launch Notes",
        "",
        spec.emulator_notes,
        "",
        "## Verified Emulator Status",
        "",
        status_table,
        "",
        "Here, `Verified` means the current package survived the repo's "
        "DOSBox-X smoke launch from `/tmp` without the old INT 6 / GPF failures.",
        "",
        "The included `CHAIN.TXT` is intentionally a local-console file. Its key values are:",
        "",
        "- `remote = 0`",
        "- `user baud = 0`",
        "- `COM port = 0`",
        "- `COM baud = 0`",
        "",
        "This is a local-console dropfile, not a remote-modem one.",
        "",
        "Important launch rules:",
        "",
        "- mount `game/` as `C:`",
        "- run plain `ECGAME`",
        "- do not use `/L`",
        "- do not pass `C:\\CHAIN.TXT` explicitly",
        "",
        "## Known-Good DOSBox-X Command",
        "",
        "Run this from the unpacked package root:",
        "",
        "```bash",
        "dosbox-x \\",
        "  -defaultconf \\",
        "  -nopromptfolder \\",
        '  -set "dosv=off" \\',
        '  -set "machine=vgaonly" \\',
        '  -set "core=normal" \\',
        '  -set "cputype=386_prefetch" \\',
        '  -set "cycles=fixed 3000" \\',
        '  -set "xms=false" \\',
        '  -set "ems=false" \\',
        '  -set "umb=false" \\',
        '  -set "output=surface" \\',
        '  -c "mount c $PWD/game" \\',
        '  -c "c:" \\',
        '  -c "mode co80" \\',
        '  -c "ECGAME"',
        "```",
        "",
        "## Baseline State",
        "",
        "The bundled game directory opens into the early join/new-player path rather than a pre-played campaign.",
    ]
    return "\n".join(lines) + "\n"


def releases_readme() -> str:
    bundle_table = markdown_table(
        ("Bundle", "Executables", "DOSBox-X", "dosemu2", "Notes"),
        [
            (
                f"`{spec.archive_name}`",
                spec.bundle_summary,
                spec.dosbox_status,
                spec.dosemu_status,
                spec.dosbox_notes,
            )
            for spec in PACKAGE_SPECS
        ],
    )
    return (
        "# Release Bundles\n\n"
        "This directory holds demo-ready Esterian Conquest zip packages for "
        "emulator testing and operator reproduction.\n\n"
        f"{bundle_table}\n\n"
        "Here, `Verified` means the current package survived the repo's "
        "DOSBox-X smoke launch from `/tmp` without the old INT 6 / GPF failures.\n\n"
        "Both archives are generated by:\n\n"
        "```bash\n"
        "python3 scripts/build_release_packages.py --verify\n"
        "```\n\n"
        "The unlocked package build refreshes `EC_UNLOCKED/` first via:\n\n"
        "```bash\n"
        "python3 tools/unlzexe/rebuild_unlocked.py --verify\n"
        "```\n\n"
        "The bundled game directory is built from the preserved "
        "`fixtures/ecutil-init/v1.5` baseline so package generation does not "
        "depend on mutable working copies under `original/v1.5/`.\n"
    )


def stage_package(spec: PackageSpec, workspace_root: Path) -> Path:
    package_root = workspace_root / f"ec-v1.5-{spec.slug}-demo"
    game_dir = package_root / "game"
    docs_dir = package_root / "docs"
    dropfiles_dir = package_root / "dropfiles"

    copy_tree_contents(FIXTURE_GAME_DIR, game_dir)
    copy_named_files(spec.exe_root, game_dir, ("ECGAME.EXE", "ECMAINT.EXE", "ECUTIL.EXE"))
    copy_named_files(ORIGINAL_DIR, game_dir, GAME_STATIC_NAMES)
    copy_named_files(ORIGINAL_DIR, docs_dir, DOC_NAMES)

    chain_path = game_dir / "CHAIN.TXT"
    write_chain_txt(chain_path, player_number=1, alias="SYSOP", real_name="SYSOP")
    dropfiles_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(chain_path, dropfiles_dir / "CHAIN.TXT")

    (package_root / "README.md").write_text(package_readme(spec, package_root), encoding="utf-8")
    return package_root


def write_zip_from_tree(tree_root: Path, zip_path: Path) -> None:
    zip_path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as zf:
        for path in sorted(tree_root.rglob("*"), key=lambda p: p.relative_to(tree_root).as_posix()):
            if path.is_dir():
                continue
            rel_path = path.relative_to(tree_root.parent).as_posix()
            info = zipfile.ZipInfo(rel_path, ZIP_TIMESTAMP)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            zf.writestr(info, path.read_bytes())


def expected_archive_entries(package_root_name: str) -> set[str]:
    entries = {
        f"{package_root_name}/README.md",
        f"{package_root_name}/dropfiles/CHAIN.TXT",
        f"{package_root_name}/game/CHAIN.TXT",
        f"{package_root_name}/game/ECGAME.EXE",
        f"{package_root_name}/game/ECMAINT.EXE",
        f"{package_root_name}/game/ECUTIL.EXE",
    }
    entries.update(f"{package_root_name}/game/{name}" for name in GAME_DAT_NAMES)
    entries.update(f"{package_root_name}/game/{name}" for name in GAME_STATIC_NAMES)
    entries.update(f"{package_root_name}/docs/{name}" for name in DOC_NAMES)
    return entries


def verify_archive(spec: PackageSpec, zip_path: Path) -> None:
    package_root_name = f"ec-v1.5-{spec.slug}-demo"
    expected_entries = expected_archive_entries(package_root_name)
    with zipfile.ZipFile(zip_path) as zf:
        names = set(zf.namelist())
        missing = sorted(expected_entries - names)
        if missing:
            raise SystemExit(f"{zip_path.name}: missing entries: {missing}")

        chain_text = zf.read(f"{package_root_name}/game/CHAIN.TXT").decode("ascii")
        chain_lines = chain_text.splitlines()
        expected_pairs = {
            14: "0",
            19: "0",
            20: "0",
            30: "0",
        }
        for index, expected in expected_pairs.items():
            actual = chain_lines[index]
            if actual != expected:
                raise SystemExit(
                    f"{zip_path.name}: CHAIN.TXT line {index + 1} expected {expected!r}, got {actual!r}"
                )

        dropfile_copy = zf.read(f"{package_root_name}/dropfiles/CHAIN.TXT")
        game_dropfile = zf.read(f"{package_root_name}/game/CHAIN.TXT")
        if dropfile_copy != game_dropfile:
            raise SystemExit(f"{zip_path.name}: game/dropfiles CHAIN.TXT copies differ")

        for exe_name in ("ECGAME.EXE", "ECMAINT.EXE", "ECUTIL.EXE"):
            archive_bytes = zf.read(f"{package_root_name}/game/{exe_name}")
            source_bytes = (spec.exe_root / exe_name).read_bytes()
            if archive_bytes != source_bytes:
                raise SystemExit(f"{zip_path.name}: {exe_name} does not match source bytes")


def build_packages(specs: list[PackageSpec], output_dir: Path, verify: bool) -> list[Path]:
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "README.md").write_text(releases_readme(), encoding="utf-8")

    built_archives: list[Path] = []
    with tempfile.TemporaryDirectory(prefix="ec-release-packages-") as temp_dir:
        workspace_root = Path(temp_dir)
        for spec in specs:
            package_root = stage_package(spec, workspace_root)
            zip_path = output_dir / spec.archive_name
            write_zip_from_tree(package_root, zip_path)
            if verify:
                verify_archive(spec, zip_path)
            built_archives.append(zip_path)
    return built_archives


def main() -> None:
    args = parse_args()
    specs = selected_specs(args.variant)
    refresh_unlocked_bundle(specs)
    archives = build_packages(specs, args.output_dir, args.verify)
    for archive in archives:
        print(archive)


if __name__ == "__main__":
    main()
