import sys
sys.path.insert(0, 'tools/unlzexe')
from emu8086 import *

def main():
    exe_path = 'original/v1.5/ECGAME.EXE'
    dump_path = 'tools/unlzexe/ecgame_ivt_live.bin'
    
    emu = Emu8086()
    load_dos_memory(emu, dump_path, 1024)
    load_seg = 0x1010
    mz = load_exe(emu, exe_path, load_seg)
    init_cs = emu.sregs['cs']
    stub_seg = init_cs
    
    try:
        while emu.insn_count < 2000000:
            cs = emu.sregs['cs']
            if cs != stub_seg:
                break
            emu.step()
    except Exception as e:
        print("Error:", e)
        
    print(f"CS changed after {emu.insn_count} insns")
    
    base = init_cs << 4
    lenlz = emu.mem[base + 0x1A9] | (emu.mem[base + 0x1AA] << 8)
    decalage = emu.mem[base + 0x1AB] | (emu.mem[base + 0x1AC] << 8)
    lenprog = emu.mem[base + 0x1A7] | (emu.mem[base + 0x1A8] << 8)
    
    print(f"lenlz: 0x{lenlz:04x}")
    print(f"decalage: 0x{decalage:04x}")
    print(f"lenprog: 0x{lenprog:04x}")
    
    # Dump the "EAT SHIT" string from the emulated memory
    s = bytes(emu.mem[base + 0x77 : base + 0x87])
    print(f"Decrypted string at 0x77: {s!r}")

if __name__ == '__main__':
    main()
