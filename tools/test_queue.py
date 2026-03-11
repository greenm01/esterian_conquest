import os
import shutil
import subprocess

def patch_file(path, offset, data):
    with open(path, 'r+b') as f:
        f.seek(offset)
        f.write(bytes(data))

target = "/tmp/test-queue"
if os.path.exists(target):
    shutil.rmtree(target)
shutil.copytree("fixtures/ecutil-init/v1.5", target)
shutil.copy2("original/v1.5/ECMAINT.EXE", target)

planets = os.path.join(target, "PLANETS.DAT")
# 100 stored goods
patch_file(planets, 1358 + 0x0A, [100, 0, 0, 0])
# Build qty: Slot 1=3, Slot 2=4, Slot 3=5
patch_file(planets, 1358 + 0x24, [3, 4, 5])
# Build ID: Slot 1=10, Slot 2=11, Slot 3=12
patch_file(planets, 1358 + 0x2E, [10, 11, 12])

# Run ECMAINT
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

print("Done")
