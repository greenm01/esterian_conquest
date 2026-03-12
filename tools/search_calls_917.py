import sys

def find_calls(filename):
    with open(filename, 'rb') as f:
        data = f.read()

    # Search for Far Calls to 3374:0917
    far_call = b'\x9a\x17\x09\x74\x33'
    idx = 0
    print("Calls to 3374:0917:")
    while True:
        idx = data.find(far_call, idx)
        if idx == -1: break
        print(f"Found Far Call at file offset 0x{idx:x}")
        idx += 1

if __name__ == '__main__':
    find_calls("/tmp/ecmaint-debug/MEMDUMP.BIN")
