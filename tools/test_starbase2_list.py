import os
import shutil
import subprocess

def patch_file(path, offset, data):
    with open(path, 'r+b') as f:
        f.seek(offset)
        f.write(bytes(data))

target = "/tmp/test-starbase2-list"
if os.path.exists(target):
    shutil.rmtree(target)
shutil.copytree("fixtures/ecmaint-starbase-pre/v1.5", target)
shutil.copy2("original/v1.5/ECMAINT.EXE", target)

patch_file(os.path.join(target, 'PLANETS.DAT'), 12 * 97 + 92, [0x02, 0x01])
patch_file(os.path.join(target, 'PLAYER.DAT'), 0x40, [0x02, 0x00])
patch_file(os.path.join(target, 'PLAYER.DAT'), 0x44, [0x02, 0x00, 0x02, 0x00])

# base_record_1 original: 0100 0100 0100 0001 0000 0010 0d...
# If 0x00=id, 0x02=owner, 0x04=starbase_id, 0x07=prev, 0x08=next
# Base 1: id=1, owner=1, sbid=1, prev=0, next=2
base_record_1 = bytearray(bytes.fromhex('0100 0100 0100 0001 0000 0010 0d80 0000 0000 0080 0000 0000 0081 0000 0000 0000 100d 01'.replace(' ', '')))
base_record_1[0x07] = 0x00
base_record_1[0x08] = 0x02

# Base 2: id=2, owner=1, sbid=2, prev=1, next=0, coords=(4,13)=0x04, 0x0d
base_record_2 = bytearray(bytes.fromhex('0100 0100 0100 0001 0000 0010 0d80 0000 0000 0080 0000 0000 0081 0000 0000 0000 100d 01'.replace(' ', '')))
base_record_2[0x00] = 0x02
base_record_2[0x04] = 0x02
base_record_2[0x07] = 0x01
base_record_2[0x08] = 0x00
base_record_2[0x0B] = 0x04 # X
base_record_2[0x0C] = 0x0D # Y
base_record_2[0x1E] = 0x04 # target X
base_record_2[0x1F] = 0x0D # target Y

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
