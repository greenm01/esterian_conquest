import os
import subprocess

# We will run ECGAME locally to test exactly what parameters it requires.
# We also have door info formats. CHAIN.TXT is a known door dropfile.
cmd = [
    "dosbox-x", "-defaultconf", "-nopromptfolder", 
    "-defaultdir", "/tmp/starbase-ready",
    "-set", "dosv=off", "-set", "machine=vgaonly", 
    "-set", "core=normal", "-set", "cputype=386_prefetch", 
    "-set", "cycles=fixed 3000",
    "-c", "mount c /tmp/starbase-ready", "-c", "c:", "-c", "ECGAME C:\\CHAIN.TXT"
]
env = os.environ.copy()
env["SDL_VIDEODRIVER"] = "wayland"
subprocess.run(cmd, env=env)
