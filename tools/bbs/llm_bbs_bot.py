import sys
import os
import time
import socket
import argparse

try:
    import pyte
except ImportError:
    print("Error: 'pyte' is not installed in this environment. Run: source /tmp/venv/bin/activate")
    sys.exit(1)

def strip_ansi_and_telnet(data: bytes) -> bytes:
    """Strips Telnet negotiation bytes so Pyte can parse pure ANSI."""
    clean_data = bytearray()
    i = 0
    while i < len(data):
        if data[i] == 255: # IAC 
            i += 3
        else:
            clean_data.append(data[i])
            i += 1
    return bytes(clean_data)

def main():
    parser = argparse.ArgumentParser(description="LLM Bot that plays EC via Enigma BBS")
    parser.add_argument("--host", default="127.0.0.1", help="Enigma BBS Host")
    parser.add_argument("--port", type=int, default=8888, help="Enigma BBS Telnet Port")
    parser.add_argument("--user", default="mag", help="BBS Username")
    parser.add_argument("--password", default="fooBar", help="BBS Password")
    args = parser.parse_args()

    print(f"1. Connecting to Enigma BBS at {args.host}:{args.port}...")
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(2.0)
    
    try:
        sock.connect((args.host, args.port))
    except Exception as e:
        print(f"Failed to connect to BBS: {e}")
        return

    # Basic setup
    screen = pyte.Screen(80, 25)
    stream = pyte.Stream(screen)

    # 2. Handle BBS Login sequence blindly with delays
    # Enigma usually expects some negotiation and then username/password
    print("2. Sending login credentials...")
    time.sleep(1) # wait for connect
    
    # Empty enter to wake up terminal
    sock.send(b"\r\n")
    time.sleep(1)
    
    # Send username
    sock.send(args.user.encode("ascii") + b"\r\n")
    time.sleep(1)
    
    # Send password
    sock.send(args.password.encode("ascii") + b"\r\n")
    time.sleep(2) # Wait for BBS to process login and draw main menu

    # 3. Launch the Door
    print("3. Launching Esterian Conquest (Command: 'EC')...")
    # Assuming 'EC' is the hotkey on the main menu to launch the door
    sock.send(b"EC\r\n")
    
    print("4. Waiting 10 seconds for DOSBox to spin up and render...")
    time.sleep(10)
    
    print("5. Capturing game screen...")
    
    # Clear socket buffer of all BBS menus so we only get the game
    try:
        raw_data = bytearray()
        while True:
            data = sock.recv(4096)
            if not data:
                break
            raw_data.extend(data)
    except socket.timeout:
        pass

    if len(raw_data) == 0:
        print("Warning: Received 0 bytes from the door. Is dosbox-x crashing?")
    else:
        print(f"6. Processing {len(raw_data)} bytes of ANSI...")
        
        # Save raw dump
        with open("/tmp/bbs_door_capture.ans", "wb") as f:
            f.write(raw_data)

        # Clean and parse for LLM
        clean = strip_ansi_and_telnet(raw_data)
        stream.feed(clean.decode("cp437", errors="replace"))

        print("\n" + "="*80)
        print("WHAT THE LLM SEES (Plain Text Representation of 80x25 Screen):")
        print("="*80)
        for line in screen.display:
            print(line)
        print("="*80 + "\n")

    print("7. Disconnecting...")
    sock.close()

if __name__ == "__main__":
    main()
