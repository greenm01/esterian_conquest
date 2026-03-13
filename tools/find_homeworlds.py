from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PLANETS_DAT = ROOT / "original" / "v1.5" / "PLANETS.DAT"
RECORD_SIZE = 97

def main():
    data = PLANETS_DAT.read_bytes()
    for i in range(len(data) // RECORD_SIZE):
        record = data[i * RECORD_SIZE : (i + 1) * RECORD_SIZE]
        x, y = record[0], record[1]
        p1, p2 = record[2], record[3]
        f1, f2 = record[8], record[9]
        owner = record[0x5D]
        status = record[0x5C]
        print(f"Planet {i+1:02}: coords=({x:02},{y:02}) potential=({p1:02x},{p2:02x}) fact=({f1:02x},{f2:02x}) owner={owner} status={status}")

if __name__ == "__main__":
    main()
