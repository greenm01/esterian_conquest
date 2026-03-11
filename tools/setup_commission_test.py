import os
import shutil

target = "/tmp/starbase-ready"
if os.path.exists(target):
    shutil.rmtree(target)
shutil.copytree("fixtures/ecutil-init/v1.5", target)
shutil.copy2("original/v1.5/ECGAME.EXE", target)
shutil.copy2("original/v1.5/ECMAINT.EXE", target)

# Give planet 14 a Starbase in Stardock
def patch_file(path, offset, data):
    with open(path, 'r+b') as f:
        f.seek(offset)
        f.write(bytes(data))

planets = os.path.join(target, "PLANETS.DAT")
# Set Stardock Slot 1 Qty to 1 (0x38)
patch_file(planets, 1358 + 0x38, [1, 0, 0, 0])
# Set Stardock Slot 1 Type to 50 (0x4C)
patch_file(planets, 1358 + 0x4C, [50, 0])

# Create CHAIN.TXT for Empire 1
chain_txt = """1
sysop
Sysop
1
25
Y
N
80
24
1
1
1
1
1
1
1
1
1
1
1
1
1
1
1
1
1
1
1
1
1
1
1
"""
with open(os.path.join(target, "CHAIN.TXT"), "w") as f:
    f.write(chain_txt)

print("Scenario created in /tmp/starbase-ready")
