import os
import shutil
import subprocess

def patch_file(path, offset, data):
    with open(path, 'r+b') as f:
        f.seek(offset)
        f.write(bytes(data))

target = "/tmp/test-starbase2-linked"
if os.path.exists(target):
    shutil.rmtree(target)
shutil.copytree("fixtures/ecmaint-starbase-pre/v1.5", target)
shutil.copy2("original/v1.5/ECMAINT.EXE", target)

patch_file(os.path.join(target, 'PLANETS.DAT'), 0 + 92, [0x02, 0x01])
patch_file(os.path.join(target, 'PLAYER.DAT'), 0x40, [0x02, 0x00])
patch_file(os.path.join(target, 'PLAYER.DAT'), 0x44, [0x02, 0x00, 0x02, 0x00])

# base_record_1: 0x03 changed from 00 to 02 (next = 2)
# base_record_1: 0x07 is 00 (prev = 0)? Let's leave it as 01 for now.
base_record_1 = bytes.fromhex('0100 0102 0100 0001 0000 0010 0d80 0000 0000 0080 0000 0000 0081 0000 0000 0000 100d 01'.replace(' ', ''))
# base_record_2: 0x03 is 00 (next = 0). 0x07 is 01 (prev = 1)?
base_record_2 = bytes.fromhex('0200 0100 0200 0001 0000 000b 0180 0000 0000 0080 0000 0000 0081 0000 0000 0000 0b01 01'.replace(' ', ''))

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
