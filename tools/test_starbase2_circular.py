import os
import shutil
import subprocess

def patch_file(path, offset, data):
    with open(path, 'r+b') as f:
        f.seek(offset)
        f.write(bytes(data))

target = "/tmp/test-starbase2-circular"
if os.path.exists(target):
    shutil.rmtree(target)
shutil.copytree("fixtures/ecmaint-starbase-pre/v1.5", target)
shutil.copy2("original/v1.5/ECMAINT.EXE", target)

patch_file(os.path.join(target, 'PLANETS.DAT'), 12 * 97 + 92, [0x02, 0x01])
patch_file(os.path.join(target, 'PLAYER.DAT'), 0x40, [0x02, 0x00])
patch_file(os.path.join(target, 'PLAYER.DAT'), 0x44, [0x02, 0x00, 0x02, 0x00])

# Base 1: Next=2 (0x03), ID=1 (0x04), Prev=2 (0x07)
base_record_1 = bytes.fromhex('0100 0102 0100 0002 0000 0010 0d80 0000 0000 0080 0000 0000 0081 0000 0000 0000 100d 01'.replace(' ', ''))
# Base 2: Slot=2 (0x00), Group=1 (0x02), Next=1 (0x03), ID=2 (0x04), Prev=1 (0x07)
base_record_2 = bytes.fromhex('0200 0101 0200 0001 0000 0004 0d80 0000 0000 0080 0000 0000 0081 0000 0000 0000 040d 01'.replace(' ', ''))

with open(os.path.join(target, 'BASES.DAT'), 'wb') as f:
    f.write(base_record_1)
    f.write(base_record_2)

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
