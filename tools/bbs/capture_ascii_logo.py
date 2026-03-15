import sys
import os
import time
import argparse
import pexpect

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
    parser.add_argument("--out", default="capture/ec_ascii_logo.txt")
    args = parser.parse_args()

    os.makedirs(os.path.dirname(args.out), exist_ok=True)

    print(f"Connecting to Enigma BBS via pexpect...")
    child = pexpect.spawn(f"telnet {args.host} {args.port}")
    
    # Wait for the BBS to start sending its banner
    try:
        child.expect(b"version", timeout=5)
    except pexpect.TIMEOUT:
        pass
    time.sleep(2)
    
    child.send(b"\r")
    time.sleep(2)
    
    child.send(args.user.encode('ascii') + b"\r")
    time.sleep(1)
    
    child.send(args.password.encode('ascii') + b"\r")
    time.sleep(1)
    
    child.send(b"\r")
    
    print("Waiting for EC ANSI prompt...")
    try:
        child.expect(b"Y/\\[N\\]", timeout=10)
        print("Got ANSI prompt. Sending N...")
        child.send(b"N\r")
    except pexpect.TIMEOUT:
        print("Timed out waiting for ANSI prompt.")
        child.close()
        return

    captured_data = bytearray()
    print("Capturing intro...")
    try:
        child.expect(b"\\(Press Return\\)", timeout=15)
        captured_data.extend(child.before)
        captured_data.extend(b"(Press Return)")
    except pexpect.TIMEOUT:
        print("Timed out waiting for (Press Return). Saving what we got...")
        captured_data.extend(child.before)

    cleaned = strip_telnet(bytes(captured_data)).decode('cp437', errors='ignore')
    
    with open(args.out, "w", encoding='utf-8') as f:
        f.write(cleaned)
        
    print(f"Saved to {args.out}")
    print("\n" + "="*80)
    print(cleaned)
    print("="*80)
    
    child.close()

if __name__ == "__main__":
    main()
