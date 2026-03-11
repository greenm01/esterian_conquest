import os
lines = [
    "COM1:", "19200", "8", "N", "1", "1", "Y", "Y", "Y", "Y",
    "Sysop", "HANNIBAL", "Orlando, FL", "123-456-7890", "1", "0",
    "100", "9000", "1", "99", "90", "90", "90", "90", "90", "90",
    "90", "90", "90", "90", "90", "90", "90", "90", "90", "90",
    "90", "90", "90", "90", "90", "90", "90", "90", "90", "90",
    "90", "90", "90", "90", "90", "90", "90", "90", "90", "90"
]
with open("/tmp/starbase-ready/DOOR.SYS", "wb") as f:
    f.write(("\r\n".join(lines) + "\r\n").encode("ascii"))

chain = [
    "1", "HANNIBAL", "Sysop", "1", "25", "Y", "N", "80", "24",
    "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1"
]
with open("/tmp/starbase-ready/CHAIN.TXT", "wb") as f:
    f.write(("\r\n".join(chain) + "\r\n").encode("ascii"))

if os.path.exists("/tmp/starbase-ready/DORINFO1.DEF"):
    os.remove("/tmp/starbase-ready/DORINFO1.DEF")
