import os
import subprocess

cmd = [
    "dosbox-x", "-defaultconf", "-nopromptfolder", 
    "-defaultdir", "/tmp/starbase-ready",
    "-set", "dosv=off", "-set", "machine=vgaonly", 
    "-set", "core=normal", "-set", "cputype=386_prefetch", 
    "-set", "cycles=fixed 3000",
    "-c", "mount c /tmp/starbase-ready", "-c", "c:", "-c", "ECGAME /L > OUT.TXT", "-c", "exit"
]
env = os.environ.copy()
env["SDL_VIDEODRIVER"] = "dummy"
subprocess.run(cmd, env=env)
