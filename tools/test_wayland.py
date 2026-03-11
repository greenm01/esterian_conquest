import os
import subprocess

cmd = [
    "dosbox-x", "-defaultconf", "-nopromptfolder", 
    "-defaultdir", "/tmp/starbase-ready",
    "-c", "mount c /tmp/starbase-ready", "-c", "c:", "-c", "ECGAME"
]
env = os.environ.copy()
env["SDL_VIDEODRIVER"] = "x11"
# Let's try x11 just to see if XWayland is alive, if wayland is too buggy
subprocess.run(cmd, env=env)
