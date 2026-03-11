import os
import shutil
import subprocess

def patch_file(path, offset, data):
    with open(path, 'r+b') as f:
        f.seek(offset)
        f.write(bytes(data))

target = "/tmp/test-planet-integrity"
if os.path.exists(target):
    shutil.rmtree(target)
shutil.copytree("fixtures/ecutil-init/v1.5", target)
shutil.copy2("original/v1.5/ECMAINT.EXE", target)

# Patch PLANETS.DAT Record 0 (offset 0*97 = 0) to be owned by Empire 1
patch_file(os.path.join(target, 'PLANETS.DAT'), 0 + 92, [0x02, 0x01])

# Patch PLAYER.DAT Empire 1 planet count
patch_file(os.path.join(target, 'PLAYER.DAT'), 0x40, [0x02, 0x00])

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
