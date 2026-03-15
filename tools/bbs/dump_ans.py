import socket
import time
import sys

host = "127.0.0.1"
port = 8888
user = "mag"
password = "fooBar"

print("1. Connecting to Enigma BBS...")
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(0.5)
sock.connect((host, port))

def drain():
    try:
        while True:
            d = sock.recv(4096)
            if not d: break
            sys.stdout.buffer.write(d)
    except:
        pass

print("2. Sleeping 2 seconds to let BBS render...")
time.sleep(2)
drain()

print("4. Sending username...")
sock.send(user.encode("ascii") + b"\r\n")
time.sleep(1)
drain()

print("5. Sending password...")
sock.send(password.encode("ascii") + b"\r\n")
time.sleep(2)
drain()

print("6. Sending EC command to launch game...")
sock.send(b"EC\r\n")
time.sleep(2)
drain()

print("Done")
