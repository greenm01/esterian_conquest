import sys
import os
import time
import argparse
import pexpect
import re

def strip_telnet(data: bytes) -> bytes:
    clean_data = bytearray()
    i = 0
    while i < len(data):
        if data[i] == 255: # IAC 
            if i + 1 < len(data):
                cmd = data[i+1]
                if cmd in (251, 252, 253, 254): # WILL, WONT, DO, DONT
                    i += 3
                elif cmd == 250: # SB (Subnegotiation)
                    end_sb = data.find(bytes([255, 240]), i)
                    if end_sb != -1:
                        i = end_sb + 2
                    else:
                        i += 2 
                else:
                    i += 2
            else:
                i += 1
        else:
            clean_data.append(data[i])
            i += 1
    return bytes(clean_data)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8888)
    parser.add_argument("--user", default="mag")
    parser.add_argument("--password", default="fooBar")
    parser.add_argument("--out", default="capture/ec_full_intro.ans")
    args = parser.parse_args()

    os.makedirs(os.path.dirname(args.out), exist_ok=True)

    print(f"Connecting to Enigma BBS via pexpect...")
    child = pexpect.spawn(f"telnet {args.host} {args.port}")
    
    # Wait for the BBS to start sending its banner
    try:
        child.expect(b"version", timeout=5)
    except pexpect.TIMEOUT:
        print("Timeout waiting for 'version'. Proceeding anyway...")
    time.sleep(2)
    
    print("Sending login sequence...")
    child.send(b"\r")
    time.sleep(2)
    
    child.send(args.user.encode('ascii') + b"\r")
    time.sleep(1)
    
    child.send(args.password.encode('ascii') + b"\r")
    time.sleep(1)
    
    child.send(b"\r")
    
    # Wait for the ANSI prompt from EC Door
    print("Waiting for EC ANSI prompt...")
    try:
        child.expect(b"Y/\\[N\\]", timeout=10)
        print("Got ANSI prompt. Requesting ANSI...")
        child.send(b"Y\r")
    except pexpect.TIMEOUT:
        print("Timed out waiting for ANSI prompt. Outputting child.before:")
        print(child.before.decode('utf-8', errors='ignore'))
        child.close()
        return

    # Now capture the intro sequence until "Press Return"
    captured_data = bytearray()
    print("Capturing intro animation...")
    try:
        child.expect(b"\\(Press Return\\)", timeout=15)
        captured_data.extend(child.before)
        captured_data.extend(b"(Press Return)")
        print("Intro captured successfully.")
    except pexpect.TIMEOUT:
        print("Timed out waiting for (Press Return). Saving what we got...")
        captured_data.extend(child.before)

    print("Cleaning telnet codes and saving...")
    cleaned_ansi = strip_telnet(bytes(captured_data))
    
    with open(args.out, "wb") as f:
        f.write(cleaned_ansi)
        
    print(f"Saved to {args.out}")
    child.close()

if __name__ == "__main__":
    main()
