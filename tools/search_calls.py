import sys

def find_calls(filename, target_offset):
    with open(filename, 'rb') as f:
        data = f.read()

    # Search for Far Calls (9A)
    # The format is 9A offset_low offset_high segment_low segment_high
    # Let's search the whole file for 9A 2C 05 59 31
    far_call = b'\x9a\x2c\x05\x59\x31'
    idx = 0
    while True:
        idx = data.find(far_call, idx)
        if idx == -1: break
        print(f"Found Far Call to 3159:052C at file offset 0x{idx:x}")
        idx += 1
        
    far_call2 = b'\x9a\x7c\x99\x00\x20'
    idx = 0
    while True:
        idx = data.find(far_call2, idx)
        if idx == -1: break
        print(f"Found Far Call to 2000:997C at file offset 0x{idx:x}")
        idx += 1

    # Search for near calls (E8) in the CS segment
    # CS segment file offset: 0x29450
    # Length: up to end of file
    start_offset = 0x29450
    for idx in range(start_offset, len(data) - 3):
        if data[idx] == 0xe8:
            offset = int.from_bytes(data[idx+1:idx+3], byteorder='little', signed=True)
            # The IP after the call instruction is (idx - start_offset) + 3
            ip_after = (idx - start_offset) + 3
            target = (ip_after + offset) & 0xFFFF
            if target == target_offset:
                print(f"Found Near Call (E8) to 0x{target_offset:04x} at file offset 0x{idx:x} (IP: 0x{idx-start_offset:04x})")

if __name__ == '__main__':
    find_calls("/tmp/ecmaint-debug/MEMDUMP.BIN", 0x052c)
