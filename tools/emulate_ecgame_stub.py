#!/usr/bin/env python3
from pathlib import Path
import sys

from unicorn import (
    Uc,
    UcError,
    UC_ARCH_X86,
    UC_MODE_16,
    UC_HOOK_CODE,
    UC_HOOK_MEM_READ_UNMAPPED,
    UC_HOOK_MEM_WRITE_UNMAPPED,
    UC_HOOK_MEM_FETCH_UNMAPPED,
)
from unicorn.x86_const import *


MEM_BASE = 0x00000
MEM_SIZE = 0x400000
PSP_SEG = 0x1000
LOAD_SEG = PSP_SEG + 0x0010


def seg_off_to_linear(seg: int, off: int) -> int:
    return (seg << 4) + off


def read_u16(data: bytes, off: int) -> int:
    return data[off] | (data[off + 1] << 8)


def write_u16(buf: bytearray, off: int, value: int) -> None:
    buf[off] = value & 0xFF
    buf[off + 1] = (value >> 8) & 0xFF


def load_words(data: bytes, off: int, count: int) -> list[int]:
    return [read_u16(data, off + i * 2) for i in range(count)]


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: emulate_ecgame_stub.py <input-exe> <output-bin>")
        return 1

    src_path = Path(sys.argv[1])
    out_path = Path(sys.argv[2])
    data = src_path.read_bytes()

    mz = load_words(data, 0, 16)
    if mz[0] != 0x5A4D:
        print("not an MZ executable")
        return 1

    load_off = mz[4] << 4
    image = data[load_off:]

    init_ip = mz[0x0A]
    init_cs = mz[0x0B]
    init_sp = mz[0x08]
    init_ss = mz[0x07]

    stub_seg = LOAD_SEG + init_cs
    stub_ip = mz[0x0A]
    stub_linear = seg_off_to_linear(stub_seg, stub_ip)

    uc = Uc(UC_ARCH_X86, UC_MODE_16)
    uc.mem_map(MEM_BASE, MEM_SIZE)
    # DOS loads the EXE after a 256-byte PSP at PSP:0000.
    uc.mem_write(seg_off_to_linear(LOAD_SEG, 0), image)

    cs = LOAD_SEG + init_cs
    ip = init_ip
    ss = LOAD_SEG + init_ss
    sp = init_sp
    ds = PSP_SEG
    es = PSP_SEG

    uc.reg_write(UC_X86_REG_CS, cs)
    uc.reg_write(UC_X86_REG_IP, ip)
    uc.reg_write(UC_X86_REG_SS, ss)
    uc.reg_write(UC_X86_REG_SP, sp)
    uc.reg_write(UC_X86_REG_DS, ds)
    uc.reg_write(UC_X86_REG_ES, es)

    state = {
        "done": False,
        "entry_seg": None,
        "entry_off": None,
        "outside_seen": 0,
        "zero_run": 0,
        "insns": 0,
    }

    stub_len = len(image) - (init_cs << 4)

    def hook_code(uc_obj, address, size, _user):
        cs_now = uc_obj.reg_read(UC_X86_REG_CS)
        ip_now = uc_obj.reg_read(UC_X86_REG_IP)
        ds_now = uc_obj.reg_read(UC_X86_REG_DS)
        es_now = uc_obj.reg_read(UC_X86_REG_ES)
        bp_now = uc_obj.reg_read(UC_X86_REG_BP)
        sp_now = uc_obj.reg_read(UC_X86_REG_SP)

        state["insns"] += 1
        code = bytes(uc_obj.mem_read(address, min(size + 8, 16)))

        if cs_now == stub_seg and 0x016C <= ip_now <= 0x01C8:
            patch = uc_obj.mem_read(seg_off_to_linear(cs_now, 0x01AF), 0x22)
            print(
                f"trace {cs_now:04x}:{ip_now:04x} ds={ds_now:04x} es={es_now:04x} "
                f"bp={bp_now:04x} sp={sp_now:04x} patch={patch.hex()}"
            )

        if cs_now == stub_seg and 0x01B0 <= ip_now <= 0x0210:
            print(
                f"handoff {cs_now:04x}:{ip_now:04x} ds={ds_now:04x} es={es_now:04x} "
                f"bp={bp_now:04x} sp={sp_now:04x} bytes={code.hex()}"
            )

        # The decompressor entry is CS:0000 and occupies the tail stub block.
        # Stop once execution leaves that block.
        if not (cs_now == stub_seg and ip_now < stub_len):
            state["outside_seen"] += 1
            if state["outside_seen"] <= 64:
                print(
                    f"outside#{state['outside_seen']} at {cs_now:04x}:{ip_now:04x} "
                    f"ds={ds_now:04x} es={es_now:04x} bp={bp_now:04x} sp={sp_now:04x} "
                    f"bytes={code.hex()}"
                )
            if code[: min(len(code), 4)] == b"\x00" * min(len(code), 4):
                state["zero_run"] += 1
            else:
                state["zero_run"] = 0
            if state["zero_run"] >= 64 or state["outside_seen"] >= 4096:
                state["done"] = True
                state["entry_seg"] = cs_now
                state["entry_off"] = ip_now
                uc_obj.emu_stop()
            return

        # Safety valve if execution runs out of the expected DOS image.
        image_lo = seg_off_to_linear(LOAD_SEG, 0)
        if not (MEM_BASE <= address < image_lo + len(image) + 0x20000):
            print(
                f"execution escaped image: cs:ip={cs_now:04x}:{ip_now:04x} "
                f"ds={ds_now:04x} es={es_now:04x}"
            )
            uc_obj.emu_stop()

    uc.hook_add(UC_HOOK_CODE, hook_code)

    def hook_unmapped(uc_obj, access, address, size, value, _user):
        cs_now = uc_obj.reg_read(UC_X86_REG_CS)
        ip_now = uc_obj.reg_read(UC_X86_REG_IP)
        ds_now = uc_obj.reg_read(UC_X86_REG_DS)
        es_now = uc_obj.reg_read(UC_X86_REG_ES)
        ss_now = uc_obj.reg_read(UC_X86_REG_SS)
        sp_now = uc_obj.reg_read(UC_X86_REG_SP)
        print(
            f"unmapped access={access} addr={address:08x} size={size} value={value} "
            f"at {cs_now:04x}:{ip_now:04x} ds={ds_now:04x} es={es_now:04x} ss={ss_now:04x} sp={sp_now:04x}"
        )
        return False

    uc.hook_add(
        UC_HOOK_MEM_READ_UNMAPPED | UC_HOOK_MEM_WRITE_UNMAPPED | UC_HOOK_MEM_FETCH_UNMAPPED,
        hook_unmapped,
    )

    try:
        uc.emu_start(stub_linear, MEM_BASE + MEM_SIZE - 1, count=10_000_000)
    except UcError as exc:
        print(f"emulation error: {exc}")
        return 1

    if not state["done"]:
        print("stub did not transfer control to original entrypoint")
        return 1

    final_stub = uc.mem_read(seg_off_to_linear(state["entry_seg"], 0), min(len(image), 0x40))
    tail = uc.mem_read(seg_off_to_linear(stub_seg, 0x016C), 0x6A)
    dump_len = 0x100000
    dumped = uc.mem_read(MEM_BASE, dump_len)
    out_path.write_bytes(dumped)
    print(f"tail_after={tail.hex()}")
    print(f"entry_bytes={final_stub.hex()}")
    print(
        f"dumped {len(dumped)} bytes; original entry {state['entry_seg']:04x}:{state['entry_off']:04x}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
