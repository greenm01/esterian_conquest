import os
import shutil
import pexpect
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "fixtures" / "ecmaint-starbase-pre" / "v1.5"
ECMAINT = ROOT / "original" / "v1.5" / "ECMAINT.EXE"

target = Path("/tmp/ecmaint-debug-16a4")
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
    "-logfile", "/tmp/dosbox.log",
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

# Give it a bit to load and unpack
time.sleep(3)

# BP at Starbase 2 check
child.sendline("BP 2000:5EE4")
child.sendline("RUN")

time.sleep(3)

# Dump registers to logfile
child.sendline("LOG EAX EBX ECX EDX ESI EDI EBP ESP DS ES FS GS CS EIP")
# Dump memory at 16A4
child.sendline("MEMDUMPBIN 2000:16A4 16") # DGROUP might be 2000? Let's just dump from 2000 segment to see
child.sendline("MEMDUMPBIN 3FF9:16A4 16")
time.sleep(1)

child.sendline("EXIT")
time.sleep(1)
try:
    child.read()
except Exception as e:
    pass
child.close()

if os.path.exists("/tmp/dosbox.log"):
    print("Logfile contents:")
    with open("/tmp/dosbox.log") as f:
        print(f.read())

if os.path.exists("/tmp/ecmaint-debug-16a4/MEMDUMP.BIN"):
    print("Found MEMDUMP.BIN from breakpoint!")
    os.system("xxd /tmp/ecmaint-debug-16a4/MEMDUMP.BIN")


