import os

from ecgame_dropfiles import write_chain_txt, write_door_sys

write_door_sys("/tmp/starbase-ready/DOOR.SYS")
write_chain_txt("/tmp/starbase-ready/CHAIN.TXT", first_name="HANNIBAL")

if os.path.exists("/tmp/starbase-ready/DORINFO1.DEF"):
    os.remove("/tmp/starbase-ready/DORINFO1.DEF")
