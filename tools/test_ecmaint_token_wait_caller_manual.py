import pexpect
import sys
import re
import time
import os

def run_debugger():
    print("Launching dosbox-x with pexpect...")
    child = pexpect.spawn('./tools/debug_ecmaint.sh', encoding='utf-8', timeout=15)
    
    try:
        child.expect(r'CS=')
        print("Debugger ready!")
        time.sleep(1)
        
        child.sendline('BPINT 21 3D')
        time.sleep(0.5)
        
        child.sendline('RUN')
        child.expect(r'CS=', timeout=10)
        print("Unpacked! Deleting BPINT and setting breakpoints...")
        time.sleep(1)
        
        child.sendline('BPDEL *')
        time.sleep(0.5)
        
        for bp in ['BP 2814:96c4', 'BP 2814:9e1e', 'BP 2814:9cb0']:
            child.sendline(bp)
            time.sleep(0.5)
            
        child.sendline('RUN')
        # We want to capture the registers. DOSBox-X prints them when stopping.
        child.expect(r'CS=', timeout=15)
        
        # We matched CS=, we want to dump the current screen of the terminal
        child.sendline('SR')
        time.sleep(1)
        # Read the terminal screen and try to find SS= and SP=
        
        # Save output to file to analyze
        with open('/tmp/ecmaint-debug-token/REGS.TXT', 'w') as f:
            f.write(child.before + child.after + child.read_nonblocking(4096, timeout=1))
            
        print("Breakpoint hit!")
        
        time.sleep(1)
        
        # Dump full 640K RAM to MEMDUMP.BIN
        child.sendline('MEMDUMPBIN 0000:0000 9FFFF')
        time.sleep(3) # Wait for dump to finish
        
        child.sendline('EXIT')
        child.expect(pexpect.EOF, timeout=5)
        
    except Exception as e:
        print("Exception:", e)
    finally:
        child.close()
        print("Done. Look for MEMDUMP.BIN and REGS.TXT")

if __name__ == '__main__':
    run_debugger()
