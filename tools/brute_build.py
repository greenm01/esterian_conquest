import os
import shutil
import subprocess

def patch_file(path, offset, data):
    with open(path, 'r+b') as f:
        f.seek(offset)
        f.write(bytes(data))

for build_id in range(1, 21):
    target = f"/tmp/build-id-{build_id}"
    if os.path.exists(target):
        shutil.rmtree(target)
    shutil.copytree("fixtures/ecutil-init/v1.5", target)
    shutil.copy2("original/v1.5/ECMAINT.EXE", target)
    
    planets = os.path.join(target, "PLANETS.DAT")
    # 100 stored goods
    patch_file(planets, 1358 + 0x0A, [100, 0, 0, 0])
    # Build qty 1
    patch_file(planets, 1358 + 0x24, [1])
    # Build ID
    patch_file(planets, 1358 + 0x2E, [build_id])
    
    # Run ECMAINT
    cmd = [
        "xvfb-run", "-a", "dosbox-x", "-defaultconf", "-nopromptfolder", 
        "-defaultdir", target, "-set", "dosv=off", "-set", "machine=vgaonly", 
        "-set", "core=normal", "-set", "cputype=386_prefetch", 
        "-set", "cycles=fixed 3000", "-set", "xms=false", "-set", "ems=false", 
        "-set", "umb=false", "-set", "output=surface", 
        "-c", f"mount c {target}", "-c", "c:", "-c", "ECMAINT /R", "-c", "exit"
    ]
    subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    
    # Check BASES.DAT
    bases_file = os.path.join(target, "BASES.DAT")
    if os.path.exists(bases_file):
        size = os.path.getsize(bases_file)
        if size > 0:
            print(f"Build ID {build_id} created BASES.DAT of size {size}")
            
print("Done brute forcing.")
