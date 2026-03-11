import os
import shutil
import subprocess

def patch_file(path, offset, data):
    with open(path, 'r+b') as f:
        f.seek(offset)
        f.write(bytes(data))

target = "/tmp/test-emp2-starbase1"
if os.path.exists(target):
    shutil.rmtree(target)
shutil.copytree("fixtures/ecmaint-starbase-pre/v1.5", target)
shutil.copy2("original/v1.5/ECMAINT.EXE", target)

# Empire 2's planet is at (4,13), Record 12. It's already owned by Empire 2.
# We don't need to patch PLANETS.DAT because it's initialized correctly for Empire 2.

# Patch PLAYER.DAT Empire 2 (Record 1, offset 88)
# 0x44: starbase count (0 -> 1)
# 0x46: companion field (0 -> 1)
patch_file(os.path.join(target, 'PLAYER.DAT'), 88 + 0x44, [0x01, 0x00, 0x01, 0x00])

# Patch BASES.DAT to have two records
base_record_1 = bytes.fromhex('0100 0100 0100 0001 0000 0010 0d80 0000 0000 0080 0000 0000 0081 0000 0000 0000 100d 01'.replace(' ', ''))
# Base 2: ID=1, Group=2, Target=(4,13), Owner=2
base_record_2 = bytes.fromhex('0100 0200 0100 0001 0000 0004 0d80 0000 0000 0080 0000 0000 0081 0000 0000 0000 040d 02'.replace(' ', ''))
with open(os.path.join(target, 'BASES.DAT'), 'wb') as f:
    f.write(base_record_1)
    f.write(base_record_2)

# Patch CONQUEST.DAT [0x3D] to 0x01 for Empire 2
patch_file(os.path.join(target, 'CONQUEST.DAT'), 0x3D, [0x01])

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
