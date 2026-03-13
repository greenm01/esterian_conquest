from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1]
FLEETS_DAT = ROOT / "original" / "v1.5" / "FLEETS.DAT"
RECORD_SIZE = 54

def main():
    data = FLEETS_DAT.read_bytes()
    for i in range(len(data) // RECORD_SIZE):
        record = data[i * RECORD_SIZE : (i + 1) * RECORD_SIZE]
        local_slot = struct.unpack("<H", record[0:2])[0]
        owner = record[2]
        next_ptr = struct.unpack("<H", record[3:5])[0]
        fleet_id = struct.unpack("<H", record[5:7])[0]
        prev_ptr = record[7]
        max_speed = record[9]
        cur_speed = record[10]
        x, y = record[11], record[12]
        order = record[0x1F]
        target_x, target_y = record[0x20], record[0x21]
        print(f"Fleet {i+1:02}: id={fleet_id} owner={owner} loc=({x},{y}) target=({target_x},{target_y}) order={order} spd={cur_speed}/{max_speed} next={next_ptr} prev={prev_ptr}")

if __name__ == "__main__":
    main()
