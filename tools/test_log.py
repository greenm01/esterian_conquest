import os
import subprocess

conf = """
[log]
file = dosbox.log
int21 = true
fileio = true
"""
with open("/tmp/starbase-ready/dosbox.conf", "w") as f:
    f.write(conf)

cmd = [
    "dosbox-x", "-conf", "/tmp/starbase-ready/dosbox.conf", "-nopromptfolder", 
    "-defaultdir", "/tmp/starbase-ready",
    "-set", "dosv=off", "-set", "machine=vgaonly", 
    "-set", "core=normal", "-set", "cputype=386_prefetch", 
    "-set", "cycles=fixed 3000",
    "-c", "mount c /tmp/starbase-ready", "-c", "c:", "-c", "ECMAINT /R", "-c", "exit"
]
env = os.environ.copy()
env["SDL_VIDEODRIVER"] = "dummy"
subprocess.run(cmd, env=env)
