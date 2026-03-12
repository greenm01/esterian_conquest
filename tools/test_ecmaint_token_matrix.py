import os
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "fixtures" / "ecmaint-starbase-pre" / "v1.5"
ECMAINT = ROOT / "original" / "v1.5" / "ECMAINT.EXE"
ARTIFACT = ROOT / "artifacts" / "ecmaint-token-matrix.txt"

BASE_RECORD_HEX = (
    "0100010001000001000000100d80000000000080000000000081000000000000100d01"
)


def build_two_base_file() -> bytes:
    base1 = bytearray.fromhex(BASE_RECORD_HEX)
    base2 = bytearray.fromhex(BASE_RECORD_HEX)
    base1[0x08] = 0x02
    base2[0x00] = 0x02
    base2[0x02] = 0x01
    base2[0x04] = 0x02
    base2[0x05] = 0x01
    base2[0x07] = 0x01
    base2[0x0B] = 0x04
    base2[0x0C] = 0x0D
    base2[0x1E] = 0x04
    base2[0x1F] = 0x0D
    return bytes(base1) + bytes(base2)


def prepare(path: Path, token_name: str | None) -> None:
    if path.exists():
        shutil.rmtree(path)
    shutil.copytree(SRC, path)
    shutil.copy2(ECMAINT, path / "ECMAINT.EXE")

    player = bytearray((path / "PLAYER.DAT").read_bytes())
    player[0x44:0x48] = bytes([0x02, 0x00, 0x02, 0x00])
    (path / "PLAYER.DAT").write_bytes(player)
    (path / "BASES.DAT").write_bytes(build_two_base_file())
    if token_name:
        (path / token_name).write_bytes(b"")


def run_ecmaint(path: Path) -> str:
    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    cmd = [
        "dosbox-x",
        "-defaultconf",
        "-nopromptfolder",
        "-defaultdir",
        str(path),
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
        "-time-limit",
        "12",
        "-c",
        f"mount c {path}",
        "-c",
        "c:",
        "-c",
        "ECMAINT /R",
        "-c",
        "exit",
    ]
    subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env, check=False)
    if (path / "ERRORS.TXT").exists():
        return (path / "ERRORS.TXT").read_text(errors="ignore").splitlines()[0]
    return "OK"


def main() -> None:
    tokens = [
        None,
        "MAIN.TOK",
        "PLAYER.TOK",
        "PLANETS.TOK",
        "FLEETS.TOK",
        "DATABASE.TOK",
        "CONQUEST.TOK",
        "FOO.TOK",
    ]

    lines: list[str] = []
    for token in tokens:
        label = token or "none"
        path = Path("/tmp") / f"ecmaint-token-matrix-{label.lower().replace('.', '-')}"
        prepare(path, token)
        result = run_ecmaint(path)
        tok_files = sorted(p.name for p in path.glob("*.TOK"))
        lines.append(f"token={label}")
        lines.append(f"  result: {result}")
        lines.append(f"  tok_files: {', '.join(tok_files) if tok_files else '<none>'}")
        lines.append("")

    ARTIFACT.write_text("\n".join(lines))
    print(f"Wrote {ARTIFACT}")


if __name__ == "__main__":
    main()
