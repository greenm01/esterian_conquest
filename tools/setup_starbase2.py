
import os

def patch_file(path, offset, data):
    with open(path, 'r+b') as f:
        f.seek(offset)
        f.write(bytes(data))

def create_starbase_2_scenario(target_dir):
    # Copy from ecmaint-starbase-pre
    os.makedirs(target_dir, exist_ok=True)
    source_dir = 'fixtures/ecmaint-starbase-pre/v1.5'
    for filename in os.listdir(source_dir):
        with open(os.path.join(source_dir, filename), 'rb') as f_src:
            with open(os.path.join(target_dir, filename), 'wb') as f_dst:
                f_dst.write(f_src.read())

    # 1. Patch PLANETS.DAT Record 0 (offset 0*97 = 0)
    # Owner slot at +0x5D (93), status at +0x5C (92)
    patch_file(os.path.join(target_dir, 'PLANETS.DAT'), 0 + 92, [0x02, 0x01])

    # 2. Patch PLAYER.DAT Empire 1
    # 0x40: planet count (1 -> 2)
    # 0x44: starbase count (1 -> 2)
    # 0x46: companion field (1 -> 2)
    patch_file(os.path.join(target_dir, 'PLAYER.DAT'), 0x40, [0x02, 0x00])
    patch_file(os.path.join(target_dir, 'PLAYER.DAT'), 0x44, [0x02, 0x00, 0x02, 0x00])

    # 3. Patch BASES.DAT to have two records
    # Record 1 (shipped): 0100 0100 0100 0001 0000 0010 0d80 0000 0000 0080 0000 0000 0081 0000 0000 0000 100d 01
    # Record 2 (new):     0200 0100 0200 0001 0000 000b 0180 0000 0000 0080 0000 0000 0081 0000 0000 0000 0b01 01
    base_record_1 = bytes.fromhex('0100 0100 0100 0001 0000 0010 0d80 0000 0000 0080 0000 0000 0081 0000 0000 0000 100d 01'.replace(' ', ''))
    base_record_2 = bytes.fromhex('0200 0100 0200 0001 0000 000b 0180 0000 0000 0080 0000 0000 0081 0000 0000 0000 0b01 01'.replace(' ', ''))
    
    with open(os.path.join(target_dir, 'BASES.DAT'), 'wb') as f:
        f.write(base_record_1)
        f.write(base_record_2)

    print(f"Created Starbase 2 scenario in {target_dir}")

if __name__ == "__main__":
    create_starbase_2_scenario('/tmp/starbase2-test')
