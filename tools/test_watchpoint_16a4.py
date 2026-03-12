import os
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "fixtures" / "ecmaint-starbase-pre" / "v1.5"
ECMAINT = ROOT / "original" / "v1.5" / "ECMAINT.EXE"
BASE_RECORD_HEX = "0100010001000001000000100d80000000000080000000000081000000000000100d01"

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

target = Path("/tmp/ecmaint-debug-16a4")
if target.exists():
    shutil.rmtree(target)
shutil.copytree(SRC, target)
shutil.copy2(ECMAINT, target / "ECMAINT.EXE")

player = bytearray((target / "PLAYER.DAT").read_bytes())
player[0x44:0x48] = bytes([0x02, 0x00, 0x02, 0x00])
(target / "PLAYER.DAT").write_bytes(player)
(target / "BASES.DAT").write_bytes(build_two_base_file())
(target / "PLAYER.TOK").write_bytes(b"")

cmd = [
    "dosbox-x",
    "-defaultconf",
    "-nopromptfolder",
    "-defaultdir", str(target),
    "-set", "dosv=off",
    "-set", "machine=vgaonly",
    "-set", "core=normal",
    "-set", "cputype=386_prefetch",
    "-set", "cycles=fixed 3000",
    "-set", "output=surface",
    "-time-limit", "15",
    "-c", f"mount c {target}",
    "-c", "c:",
    "-c", "DEBUGBOX ECMAINT /R",
]

# Write a debug script to run when DEBUGBOX starts
debug_script = target / "DEBUG.TXT"
debug_script.write_text("""
BPINT 21 3D
RUN
BPM 39AB:16A4 W
RUN
LOG EAX EBX ECX EDX ESI EDI EBP ESP DS ES FS GS CS EIP
RUN
LOG EAX EBX ECX EDX ESI EDI EBP ESP DS ES FS GS CS EIP
RUN
""")

env = os.environ.copy()
env["SDL_VIDEODRIVER"] = "dummy"
env["SDL_AUDIODRIVER"] = "dummy"
print("Running DOSBox-X to watch 16A4...")
subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)

log_file = Path("LOG.TXT")
if log_file.exists():
    print(log_file.read_text())
else:
    print("LOG.TXT not found!")
