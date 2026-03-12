import os
import shutil
import pexpect
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "fixtures" / "ecmaint-starbase-pre" / "v1.5"
ECMAINT = ROOT / "original" / "v1.5" / "ECMAINT.EXE"
target = Path("/tmp/ecmaint-debug-toks")
if target.exists():
    shutil.rmtree(target)
shutil.copytree(SRC, target)
shutil.copy2(ECMAINT, target / "ECMAINT.EXE")

toks = ["Planets.Tok", "Fleets.Tok", "Player.Tok", "IPBMs.Tok", "Conquest.Tok", "Message.Tok", "Results.Tok", "Database.Tok", "setup.tok"]
for t in toks:
    (target / t).write_bytes(b"")

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
    "-logfile", "/tmp/dosbox_toks.log",
    "-c", f"mount c {target}",
    "-c", "c:",
    "-c", "DEBUGBOX ECMAINT /R",
]

env = os.environ.copy()
env["SDL_VIDEODRIVER"] = "dummy"
env["SDL_AUDIODRIVER"] = "dummy"
env["TERM"] = "dumb"

print("Spawning DOSBox-X...")
child = pexpect.spawn(" ".join(cmd), env=env, timeout=10, encoding='utf-8')

time.sleep(3)
child.sendline("BP 2814:5EE4")
child.sendline("RUN")
time.sleep(5)
child.sendline("MEMDUMPBIN 3FF9:16A4 16")
time.sleep(1)
child.sendline("EXIT")
time.sleep(1)

if os.path.exists("/tmp/ecmaint-debug-toks/MEMDUMP.BIN"):
    print("Found MEMDUMP.BIN!")
    os.system("xxd /tmp/ecmaint-debug-toks/MEMDUMP.BIN")
else:
    print("No dump created.")
