import os
import shutil
import subprocess

def patch_file(path, offset, data):
    with open(path, 'r+b') as f:
        f.seek(offset)
        f.write(bytes(data))

target = "/tmp/test-build-special"
if os.path.exists(target):
    shutil.rmtree(target)
shutil.copytree("fixtures/ecutil-init/v1.5", target)
shutil.copy2("original/v1.5/ECMAINT.EXE", target)

planets = os.path.join(target, "PLANETS.DAT")
# 1000 stored goods
patch_file(planets, 1358 + 0x0A, [0xE8, 0x03, 0, 0])
# Build qty
patch_file(planets, 1358 + 0x24, [1, 0, 1, 0, 1, 0])
# Build ID
patch_file(planets, 1358 + 0x2E, [8, 0, 20, 0, 50, 0])

cmd = [
    "dosbox-x", "-defaultconf", "-nopromptfolder", 
    "-defaultdir", target, "-set", "dosv=off", "-set", "machine=vgaonly", 
    "-set", "core=normal", "-set", "cputype=386_prefetch", 
    "-set", "cycles=fixed 3000", "-set", "xms=false", "-set", "ems=false", 
    "-set", "umb=false", "-set", "output=surface", 
    "-c", f"mount c {target}", "-c", "c:", "-c", "ECMAINT /R", "-c", "exit"
]
env = os.environ.copy()
env["SDL_VIDEODRIVER"] = "dummy"
subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
