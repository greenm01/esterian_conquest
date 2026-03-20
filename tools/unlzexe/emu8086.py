#!/usr/bin/env python3
"""Full 8086/186 emulator for decrypting LZEXE-wrapped DOS executables.

Handles the 'eb ff' anti-disassembly trick correctly, which Unicorn cannot.
Uses a full 1MB physical address space with proper segment:offset addressing.
Supports multi-layer self-modifying stubs (XOR decryption, code rewriting).
"""
import argparse
import struct
import sys
from pathlib import Path


class Emu8086:
    """8086/186 emulator with 1MB address space."""

    def __init__(self):
        self.mem = bytearray(0x100000)  # 1MB
        self.regs = {
            'ax': 0, 'bx': 0, 'cx': 0, 'dx': 0,
            'si': 0, 'di': 0, 'bp': 0, 'sp': 0,
        }
        self.sregs = {'cs': 0, 'ds': 0, 'es': 0, 'ss': 0}
        self.ip = 0
        self.flags = 0x0200  # IF set
        self.halted = False
        self.insn_count = 0

    # --- Register helpers ---
    R16 = ['ax', 'cx', 'dx', 'bx', 'sp', 'bp', 'si', 'di']
    R8 = ['al', 'cl', 'dl', 'bl', 'ah', 'ch', 'dh', 'bh']
    SREG_NAMES = {0: 'es', 1: 'cs', 2: 'ss', 3: 'ds'}

    def r8(self, name):
        pair, shift = {
            'al': ('ax', 0), 'cl': ('cx', 0), 'dl': ('dx', 0), 'bl': ('bx', 0),
            'ah': ('ax', 8), 'ch': ('cx', 8), 'dh': ('dx', 8), 'bh': ('bx', 8),
        }[name]
        return (self.regs[pair] >> shift) & 0xFF

    def w8(self, name, val):
        pair, mask, shift = {
            'al': ('ax', 0xFF00, 0), 'cl': ('cx', 0xFF00, 0),
            'dl': ('dx', 0xFF00, 0), 'bl': ('bx', 0xFF00, 0),
            'ah': ('ax', 0x00FF, 8), 'ch': ('cx', 0x00FF, 8),
            'dh': ('dx', 0x00FF, 8), 'bh': ('bx', 0x00FF, 8),
        }[name]
        self.regs[pair] = (self.regs[pair] & mask) | ((val & 0xFF) << shift)

    # --- Memory helpers (physical 20-bit addressing) ---
    def phys(self, seg, off):
        return ((seg << 4) + off) & 0xFFFFF

    def rb(self, seg, off):
        return self.mem[self.phys(seg, off)]

    def rw(self, seg, off):
        a = self.phys(seg, off)
        return self.mem[a] | (self.mem[(a + 1) & 0xFFFFF] << 8)

    def wb(self, seg, off, val):
        self.mem[self.phys(seg, off)] = val & 0xFF

    def ww(self, seg, off, val):
        a = self.phys(seg, off)
        self.mem[a] = val & 0xFF
        self.mem[(a + 1) & 0xFFFFF] = (val >> 8) & 0xFF

    # --- Code fetch (always from CS:IP) ---
    def fetch8(self):
        v = self.rb(self.sregs['cs'], self.ip)
        self.ip = (self.ip + 1) & 0xFFFF
        return v

    def fetch16(self):
        lo = self.fetch8()
        hi = self.fetch8()
        return lo | (hi << 8)

    def fetchs8(self):
        v = self.fetch8()
        return v - 256 if v >= 128 else v

    def fetchs16(self):
        v = self.fetch16()
        return v - 65536 if v >= 32768 else v

    # --- Stack (uses SS:SP) ---
    def push(self, val):
        self.regs['sp'] = (self.regs['sp'] - 2) & 0xFFFF
        self.ww(self.sregs['ss'], self.regs['sp'], val)

    def pop(self):
        val = self.rw(self.sregs['ss'], self.regs['sp'])
        self.regs['sp'] = (self.regs['sp'] + 2) & 0xFFFF
        return val

    # --- ModR/M decoding ---
    def _seg_for_rm(self, rm, override):
        if override is not None:
            return override
        if rm in (2, 3, 6):
            return self.sregs['ss']
        return self.sregs['ds']

    def decode_modrm(self, seg_override=None):
        modrm = self.fetch8()
        mod = (modrm >> 6) & 3
        reg = (modrm >> 3) & 7
        rm = modrm & 7

        if mod == 3:
            return mod, reg, rm, None, None

        if mod == 0 and rm == 6:
            off = self.fetch16()
            seg = seg_override if seg_override is not None else self.sregs['ds']
            return mod, reg, rm, seg, off

        bases = {
            0: lambda: (self.regs['bx'] + self.regs['si']) & 0xFFFF,
            1: lambda: (self.regs['bx'] + self.regs['di']) & 0xFFFF,
            2: lambda: (self.regs['bp'] + self.regs['si']) & 0xFFFF,
            3: lambda: (self.regs['bp'] + self.regs['di']) & 0xFFFF,
            4: lambda: self.regs['si'],
            5: lambda: self.regs['di'],
            6: lambda: self.regs['bp'],
            7: lambda: self.regs['bx'],
        }
        base = bases[rm]()

        if mod == 1:
            disp = self.fetchs8()
        elif mod == 2:
            disp = self.fetch16()
        else:
            disp = 0

        off = (base + disp) & 0xFFFF
        seg = self._seg_for_rm(rm, seg_override)
        return mod, reg, rm, seg, off

    def read_rm8(self, mod, rm, seg, off):
        if mod == 3:
            return self.r8(self.R8[rm])
        return self.rb(seg, off)

    def write_rm8(self, mod, rm, seg, off, val):
        if mod == 3:
            self.w8(self.R8[rm], val)
        else:
            self.wb(seg, off, val)

    def read_rm16(self, mod, rm, seg, off):
        if mod == 3:
            return self.regs[self.R16[rm]]
        return self.rw(seg, off)

    def write_rm16(self, mod, rm, seg, off, val):
        if mod == 3:
            self.regs[self.R16[rm]] = val & 0xFFFF
        else:
            self.ww(seg, off, val)

    # --- Flag helpers ---
    CF = 0x0001
    PF = 0x0004
    AF = 0x0010
    ZF = 0x0040
    SF = 0x0080
    TF = 0x0100
    IF = 0x0200
    DF = 0x0400
    OF = 0x0800

    def set_zf(self, val):
        if val:
            self.flags |= self.ZF
        else:
            self.flags &= ~self.ZF

    def get_zf(self):
        return bool(self.flags & self.ZF)

    def get_cf(self):
        return bool(self.flags & self.CF)

    def set_cf(self, val):
        if val:
            self.flags |= self.CF
        else:
            self.flags &= ~self.CF

    def get_df(self):
        return bool(self.flags & self.DF)

    def update_flags_sub8(self, a, b, result):
        self.set_cf(a < b)
        self.set_zf((result & 0xFF) == 0)
        if result & 0x80:
            self.flags |= self.SF
        else:
            self.flags &= ~self.SF

    def update_flags_sub16(self, a, b, result):
        self.set_cf(a < b)
        self.set_zf((result & 0xFFFF) == 0)
        if result & 0x8000:
            self.flags |= self.SF
        else:
            self.flags &= ~self.SF

    def update_flags_logic8(self, result):
        self.set_cf(False)
        self.set_zf((result & 0xFF) == 0)
        if result & 0x80:
            self.flags |= self.SF
        else:
            self.flags &= ~self.SF

    def update_flags_logic16(self, result):
        self.set_cf(False)
        self.set_zf((result & 0xFFFF) == 0)
        if result & 0x8000:
            self.flags |= self.SF
        else:
            self.flags &= ~self.SF

    # --- ALU operations ---
    def alu8(self, op, a, b):
        cf = self.get_cf()
        if op == 0: result = (a + b) & 0xFF        # ADD
        elif op == 1: result = a | b                # OR
        elif op == 2: result = (a + b + cf) & 0xFF  # ADC
        elif op == 3: result = (a - b - cf) & 0xFF  # SBB
        elif op == 4: result = a & b                # AND
        elif op == 5: result = (a - b) & 0xFF       # SUB
        elif op == 6: result = a ^ b                # XOR
        else: result = (a - b) & 0xFF               # CMP
        # Update flags
        if op in (0, 2):  # ADD, ADC
            full = a + b + (cf if op == 2 else 0)
            self.set_cf(full > 0xFF)
        elif op in (3, 5, 7):  # SBB, SUB, CMP
            full = a - b - (cf if op == 3 else 0)
            self.set_cf(full < 0)
        elif op in (1, 4, 6):  # OR, AND, XOR
            self.set_cf(False)
        self.set_zf(result == 0)
        if result & 0x80:
            self.flags |= self.SF
        else:
            self.flags &= ~self.SF
        return result

    def alu16(self, op, a, b):
        cf = self.get_cf()
        if op == 0: result = (a + b) & 0xFFFF
        elif op == 1: result = a | b
        elif op == 2: result = (a + b + cf) & 0xFFFF
        elif op == 3: result = (a - b - cf) & 0xFFFF
        elif op == 4: result = a & b
        elif op == 5: result = (a - b) & 0xFFFF
        elif op == 6: result = a ^ b
        else: result = (a - b) & 0xFFFF
        if op in (0, 2):
            full = a + b + (cf if op == 2 else 0)
            self.set_cf(full > 0xFFFF)
        elif op in (3, 5, 7):
            full = a - b - (cf if op == 3 else 0)
            self.set_cf(full < 0)
        elif op in (1, 4, 6):
            self.set_cf(False)
        self.set_zf(result == 0)
        if result & 0x8000:
            self.flags |= self.SF
        else:
            self.flags &= ~self.SF
        return result

    # --- String direction helper ---
    def strd(self, size):
        """Return +size or -size based on DF."""
        return -size if self.get_df() else size

    # --- Main execution ---
    def step(self):
        """Execute one instruction. Returns False to stop."""
        seg_override = None

        # Handle prefixes
        while True:
            op = self.fetch8()
            if op == 0x26:
                seg_override = self.sregs['es']
            elif op == 0x2E:
                seg_override = self.sregs['cs']
            elif op == 0x36:
                seg_override = self.sregs['ss']
            elif op == 0x3E:
                seg_override = self.sregs['ds']
            elif op == 0xF0:  # LOCK prefix — ignore
                pass
            else:
                break

        ds = seg_override if seg_override is not None else self.sregs['ds']

        # --- NOP ---
        if op == 0x90:
            pass

        # --- PUSH/POP r16 ---
        elif 0x50 <= op <= 0x57:
            self.push(self.regs[self.R16[op - 0x50]])
        elif 0x58 <= op <= 0x5F:
            self.regs[self.R16[op - 0x58]] = self.pop()

        # --- PUSH/POP segment ---
        elif op == 0x06: self.push(self.sregs['es'])
        elif op == 0x0E: self.push(self.sregs['cs'])
        elif op == 0x16: self.push(self.sregs['ss'])
        elif op == 0x1E: self.push(self.sregs['ds'])
        elif op == 0x07: self.sregs['es'] = self.pop()
        elif op == 0x17: self.sregs['ss'] = self.pop()
        elif op == 0x1F: self.sregs['ds'] = self.pop()

        # --- PUSHA / POPA (186+) ---
        elif op == 0x60:
            t = self.regs['sp']
            for r in ['ax', 'cx', 'dx', 'bx']:
                self.push(self.regs[r])
            self.push(t)
            self.push(self.regs['bp'])
            self.push(self.regs['si'])
            self.push(self.regs['di'])
        elif op == 0x61:
            self.regs['di'] = self.pop()
            self.regs['si'] = self.pop()
            self.regs['bp'] = self.pop()
            self.pop()  # skip SP
            self.regs['bx'] = self.pop()
            self.regs['dx'] = self.pop()
            self.regs['cx'] = self.pop()
            self.regs['ax'] = self.pop()

        # --- PUSH imm16 (186+) / PUSH imm8 (186+) ---
        elif op == 0x68:
            self.push(self.fetch16())
        elif op == 0x6A:
            self.push(self.fetchs8() & 0xFFFF)

        # --- PUSHF / POPF ---
        elif op == 0x9C:
            self.push(self.flags)
        elif op == 0x9D:
            self.flags = self.pop()

        # --- Flag manipulation ---
        elif op == 0xF8: self.set_cf(False)     # CLC
        elif op == 0xF9: self.set_cf(True)      # STC
        elif op == 0xF5: self.set_cf(not self.get_cf())  # CMC
        elif op == 0xFA: self.flags &= ~self.IF  # CLI
        elif op == 0xFB: self.flags |= self.IF   # STI
        elif op == 0xFC: self.flags &= ~self.DF  # CLD
        elif op == 0xFD: self.flags |= self.DF   # STD

        # --- HLT ---
        elif op == 0xF4:
            self.halted = True
            return False

        # --- CBW / CWD ---
        elif op == 0x98:
            al = self.r8('al')
            self.regs['ax'] = (0xFF00 | al) if al >= 0x80 else al
        elif op == 0x99:
            self.regs['dx'] = 0xFFFF if self.regs['ax'] >= 0x8000 else 0

        # --- LAHF / SAHF ---
        elif op == 0x9F:
            self.w8('ah', self.flags & 0xFF)
        elif op == 0x9E:
            self.flags = (self.flags & 0xFF00) | self.r8('ah')

        # --- WAIT/FWAIT ---
        elif op == 0x9B:
            pass

        # --- DAA/DAS/AAA/AAS (simplified) ---
        elif op in (0x27, 0x2F, 0x37, 0x3F):
            pass

        # --- AAM/AAD ---
        elif op in (0xD4, 0xD5):
            imm = self.fetch8()
            if op == 0xD4 and imm:  # AAM
                self.w8('ah', self.r8('al') // imm)
                self.w8('al', self.r8('al') % imm)
            elif op == 0xD5:  # AAD
                self.regs['ax'] = (self.r8('ah') * (imm or 10) + self.r8('al')) & 0xFF

        # --- SALC (undocumented: set AL from carry) ---
        elif op == 0xD6:
            self.w8('al', 0xFF if self.get_cf() else 0x00)

        # --- INC/DEC r16 ---
        elif 0x40 <= op <= 0x47:
            r = self.R16[op - 0x40]
            self.regs[r] = (self.regs[r] + 1) & 0xFFFF
            self.set_zf(self.regs[r] == 0)
        elif 0x48 <= op <= 0x4F:
            r = self.R16[op - 0x48]
            self.regs[r] = (self.regs[r] - 1) & 0xFFFF
            self.set_zf(self.regs[r] == 0)

        # --- XCHG AX, r16 ---
        elif 0x91 <= op <= 0x97:
            r = self.R16[op - 0x90]
            self.regs['ax'], self.regs[r] = self.regs[r], self.regs['ax']

        # --- MOV r8, imm8 / MOV r16, imm16 ---
        elif 0xB0 <= op <= 0xB7:
            self.w8(self.R8[op - 0xB0], self.fetch8())
        elif 0xB8 <= op <= 0xBF:
            self.regs[self.R16[op - 0xB8]] = self.fetch16()

        # --- ALU r/m8, r8 ---
        elif op in (0x00, 0x08, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38):
            aluop = (op >> 3) & 7
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            a = self.read_rm8(mod, rm, seg, off)
            b = self.r8(self.R8[reg])
            result = self.alu8(aluop, a, b)
            if aluop != 7:
                self.write_rm8(mod, rm, seg, off, result)

        # --- ALU r8, r/m8 ---
        elif op in (0x02, 0x0A, 0x12, 0x1A, 0x22, 0x2A, 0x32, 0x3A):
            aluop = (op >> 3) & 7
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            a = self.r8(self.R8[reg])
            b = self.read_rm8(mod, rm, seg, off)
            result = self.alu8(aluop, a, b)
            if aluop != 7:
                self.w8(self.R8[reg], result)

        # --- ALU r/m16, r16 ---
        elif op in (0x01, 0x09, 0x11, 0x19, 0x21, 0x29, 0x31, 0x39):
            aluop = (op >> 3) & 7
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            a = self.read_rm16(mod, rm, seg, off)
            b = self.regs[self.R16[reg]]
            result = self.alu16(aluop, a, b)
            if aluop != 7:
                self.write_rm16(mod, rm, seg, off, result)

        # --- ALU r16, r/m16 ---
        elif op in (0x03, 0x0B, 0x13, 0x1B, 0x23, 0x2B, 0x33, 0x3B):
            aluop = (op >> 3) & 7
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            a = self.regs[self.R16[reg]]
            b = self.read_rm16(mod, rm, seg, off)
            result = self.alu16(aluop, a, b)
            if aluop != 7:
                self.regs[self.R16[reg]] = result

        # --- ALU AL, imm8 ---
        elif op in (0x04, 0x0C, 0x14, 0x1C, 0x24, 0x2C, 0x34, 0x3C):
            aluop = (op >> 3) & 7
            a = self.r8('al')
            b = self.fetch8()
            result = self.alu8(aluop, a, b)
            if aluop != 7:
                self.w8('al', result)

        # --- ALU AX, imm16 ---
        elif op in (0x05, 0x0D, 0x15, 0x1D, 0x25, 0x2D, 0x35, 0x3D):
            aluop = (op >> 3) & 7
            a = self.regs['ax']
            b = self.fetch16()
            result = self.alu16(aluop, a, b)
            if aluop != 7:
                self.regs['ax'] = result

        # --- 80/81/82/83 group ---
        elif op in (0x80, 0x81, 0x82, 0x83):
            mod, aluop, rm, seg, off = self.decode_modrm(seg_override)
            if op in (0x80, 0x82):
                a = self.read_rm8(mod, rm, seg, off)
                b = self.fetch8()
                result = self.alu8(aluop, a, b)
                if aluop != 7:
                    self.write_rm8(mod, rm, seg, off, result)
            else:
                a = self.read_rm16(mod, rm, seg, off)
                b = self.fetch16() if op == 0x81 else (self.fetchs8() & 0xFFFF)
                result = self.alu16(aluop, a, b)
                if aluop != 7:
                    self.write_rm16(mod, rm, seg, off, result)

        # --- TEST AL, imm8 / TEST AX, imm16 ---
        elif op == 0xA8:
            result = self.r8('al') & self.fetch8()
            self.update_flags_logic8(result)
        elif op == 0xA9:
            result = self.regs['ax'] & self.fetch16()
            self.update_flags_logic16(result)

        # --- MOV r/m, r and r, r/m ---
        elif op == 0x88:
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            self.write_rm8(mod, rm, seg, off, self.r8(self.R8[reg]))
        elif op == 0x89:
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            self.write_rm16(mod, rm, seg, off, self.regs[self.R16[reg]])
        elif op == 0x8A:
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            self.w8(self.R8[reg], self.read_rm8(mod, rm, seg, off))
        elif op == 0x8B:
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            self.regs[self.R16[reg]] = self.read_rm16(mod, rm, seg, off)

        # --- MOV seg, r/m16 / r/m16, seg ---
        elif op == 0x8E:
            mod, sreg_idx, rm, seg, off = self.decode_modrm(seg_override)
            if sreg_idx in self.SREG_NAMES:
                self.sregs[self.SREG_NAMES[sreg_idx]] = self.read_rm16(mod, rm, seg, off)
        elif op == 0x8C:
            mod, sreg_idx, rm, seg, off = self.decode_modrm(seg_override)
            if sreg_idx in self.SREG_NAMES:
                self.write_rm16(mod, rm, seg, off, self.sregs[self.SREG_NAMES[sreg_idx]])

        # --- POP r/m16 (8F — all reg fields, undocumented /1-/7 work as POP) ---
        elif op == 0x8F:
            mod, _reg, rm, seg, off = self.decode_modrm(seg_override)
            val = self.pop()
            self.write_rm16(mod, rm, seg, off, val)

        # --- MOV AL/AX, [disp16] / [disp16], AL/AX ---
        elif op == 0xA0:
            self.w8('al', self.rb(ds, self.fetch16()))
        elif op == 0xA1:
            self.regs['ax'] = self.rw(ds, self.fetch16())
        elif op == 0xA2:
            self.wb(ds, self.fetch16(), self.r8('al'))
        elif op == 0xA3:
            self.ww(ds, self.fetch16(), self.regs['ax'])

        # --- C6/C7 MOV r/m, imm ---
        elif op == 0xC6:
            mod, _, rm, seg, off = self.decode_modrm(seg_override)
            self.write_rm8(mod, rm, seg, off, self.fetch8())
        elif op == 0xC7:
            mod, _, rm, seg, off = self.decode_modrm(seg_override)
            self.write_rm16(mod, rm, seg, off, self.fetch16())

        # --- LEA ---
        elif op == 0x8D:
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            if off is not None:
                self.regs[self.R16[reg]] = off

        # --- LES / LDS ---
        elif op == 0xC4:  # LES
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            if seg is not None and off is not None:
                self.regs[self.R16[reg]] = self.rw(seg, off)
                self.sregs['es'] = self.rw(seg, (off + 2) & 0xFFFF)
        elif op == 0xC5:  # LDS
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            if seg is not None and off is not None:
                self.regs[self.R16[reg]] = self.rw(seg, off)
                self.sregs['ds'] = self.rw(seg, (off + 2) & 0xFFFF)

        # --- XCHG r8,r/m8 / r16,r/m16 ---
        elif op == 0x86:
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            a = self.r8(self.R8[reg])
            b = self.read_rm8(mod, rm, seg, off)
            self.w8(self.R8[reg], b)
            self.write_rm8(mod, rm, seg, off, a)
        elif op == 0x87:
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            a = self.regs[self.R16[reg]]
            b = self.read_rm16(mod, rm, seg, off)
            self.regs[self.R16[reg]] = b
            self.write_rm16(mod, rm, seg, off, a)

        # --- TEST r/m, r ---
        elif op == 0x84:
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            result = self.read_rm8(mod, rm, seg, off) & self.r8(self.R8[reg])
            self.update_flags_logic8(result)
        elif op == 0x85:
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            result = self.read_rm16(mod, rm, seg, off) & self.regs[self.R16[reg]]
            self.update_flags_logic16(result)

        # --- F6/F7 group (TEST/NOT/NEG/MUL/IMUL/DIV/IDIV) ---
        elif op in (0xF6, 0xF7):
            mod, grp, rm, seg, off = self.decode_modrm(seg_override)
            if op == 0xF6:
                v = self.read_rm8(mod, rm, seg, off)
                if grp == 0 or grp == 1:  # TEST imm8
                    self.update_flags_logic8(v & self.fetch8())
                elif grp == 2:  # NOT
                    self.write_rm8(mod, rm, seg, off, (~v) & 0xFF)
                elif grp == 3:  # NEG
                    r = (-v) & 0xFF
                    self.set_cf(v != 0)
                    self.set_zf(r == 0)
                    self.write_rm8(mod, rm, seg, off, r)
                elif grp == 4:  # MUL
                    self.regs['ax'] = self.r8('al') * v
                    self.set_cf(self.regs['ax'] > 0xFF)
                elif grp == 5:  # IMUL
                    sa = self.r8('al')
                    if sa >= 0x80: sa -= 0x100
                    sv = v
                    if sv >= 0x80: sv -= 0x100
                    r = (sa * sv) & 0xFFFF
                    self.regs['ax'] = r
                elif grp == 6:  # DIV
                    if v:
                        self.w8('al', self.regs['ax'] // v)
                        self.w8('ah', self.regs['ax'] % v)
                elif grp == 7:  # IDIV
                    if v:
                        dividend = self.regs['ax']
                        if dividend >= 0x8000: dividend -= 0x10000
                        divisor = v
                        if divisor >= 0x80: divisor -= 0x100
                        if divisor:
                            q = int(dividend / divisor)
                            r = dividend - q * divisor
                            self.w8('al', q & 0xFF)
                            self.w8('ah', r & 0xFF)
            else:
                v = self.read_rm16(mod, rm, seg, off)
                if grp == 0 or grp == 1:  # TEST imm16
                    self.update_flags_logic16(v & self.fetch16())
                elif grp == 2:  # NOT
                    self.write_rm16(mod, rm, seg, off, (~v) & 0xFFFF)
                elif grp == 3:  # NEG
                    r = (-v) & 0xFFFF
                    self.set_cf(v != 0)
                    self.set_zf(r == 0)
                    self.write_rm16(mod, rm, seg, off, r)
                elif grp == 4:  # MUL
                    result = self.regs['ax'] * v
                    self.regs['ax'] = result & 0xFFFF
                    self.regs['dx'] = (result >> 16) & 0xFFFF
                    self.set_cf(self.regs['dx'] != 0)
                elif grp == 5:  # IMUL
                    sa = self.regs['ax']
                    if sa >= 0x8000: sa -= 0x10000
                    sv = v
                    if sv >= 0x8000: sv -= 0x10000
                    result = sa * sv
                    self.regs['ax'] = result & 0xFFFF
                    self.regs['dx'] = (result >> 16) & 0xFFFF
                elif grp == 6:  # DIV
                    if v:
                        dividend = self.regs['ax'] | (self.regs['dx'] << 16)
                        self.regs['ax'] = (dividend // v) & 0xFFFF
                        self.regs['dx'] = (dividend % v) & 0xFFFF
                elif grp == 7:  # IDIV
                    if v:
                        dividend = self.regs['ax'] | (self.regs['dx'] << 16)
                        if dividend >= 0x80000000: dividend -= 0x100000000
                        divisor = v
                        if divisor >= 0x8000: divisor -= 0x10000
                        if divisor:
                            q = int(dividend / divisor)
                            r = dividend - q * divisor
                            self.regs['ax'] = q & 0xFFFF
                            self.regs['dx'] = r & 0xFFFF

        # --- IMUL r16, r/m16, imm (186+) ---
        elif op == 0x69:
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            v = self.read_rm16(mod, rm, seg, off)
            imm = self.fetchs16()
            self.regs[self.R16[reg]] = (v * imm) & 0xFFFF
        elif op == 0x6B:
            mod, reg, rm, seg, off = self.decode_modrm(seg_override)
            v = self.read_rm16(mod, rm, seg, off)
            imm = self.fetchs8()
            self.regs[self.R16[reg]] = (v * imm) & 0xFFFF

        # --- Shift/rotate D0/D1 (by 1), D2/D3 (by CL), C0/C1 (by imm8, 186+) ---
        elif op in (0xD0, 0xD1, 0xD2, 0xD3, 0xC0, 0xC1):
            mod, grp, rm, seg, off = self.decode_modrm(seg_override)
            bits = 8 if op in (0xD0, 0xD2, 0xC0) else 16
            mask = (1 << bits) - 1
            if op in (0xD0, 0xD1):
                count = 1
            elif op in (0xD2, 0xD3):
                count = self.r8('cl') & 0x1F
            else:
                count = self.fetch8() & 0x1F

            v = self.read_rm8(mod, rm, seg, off) if bits == 8 else self.read_rm16(mod, rm, seg, off)

            for _ in range(count):
                if grp == 0:  # ROL
                    top = (v >> (bits - 1)) & 1
                    v = ((v << 1) | top) & mask
                    self.set_cf(bool(top))
                elif grp == 1:  # ROR
                    bot = v & 1
                    v = ((bot << (bits - 1)) | (v >> 1)) & mask
                    self.set_cf(bool(bot))
                elif grp == 2:  # RCL
                    top = (v >> (bits - 1)) & 1
                    v = ((v << 1) | (1 if self.get_cf() else 0)) & mask
                    self.set_cf(bool(top))
                elif grp == 3:  # RCR
                    bot = v & 1
                    v = (((1 if self.get_cf() else 0) << (bits - 1)) | (v >> 1)) & mask
                    self.set_cf(bool(bot))
                elif grp == 4:  # SHL/SAL
                    self.set_cf(bool(v & (1 << (bits - 1))))
                    v = (v << 1) & mask
                elif grp == 5:  # SHR
                    self.set_cf(bool(v & 1))
                    v >>= 1
                elif grp == 7:  # SAR
                    self.set_cf(bool(v & 1))
                    sign = v & (1 << (bits - 1))
                    v = (v >> 1) | sign

            self.set_zf(v == 0)
            if bits == 8:
                self.write_rm8(mod, rm, seg, off, v)
            else:
                self.write_rm16(mod, rm, seg, off, v)

        # --- FF group ---
        elif op == 0xFF:
            mod, grp, rm, seg, off = self.decode_modrm(seg_override)
            if grp == 0:
                v = (self.read_rm16(mod, rm, seg, off) + 1) & 0xFFFF
                self.write_rm16(mod, rm, seg, off, v)
                self.set_zf(v == 0)
            elif grp == 1:
                v = (self.read_rm16(mod, rm, seg, off) - 1) & 0xFFFF
                self.write_rm16(mod, rm, seg, off, v)
                self.set_zf(v == 0)
            elif grp == 2:  # CALL r/m16
                target = self.read_rm16(mod, rm, seg, off)
                self.push(self.ip)
                self.ip = target
            elif grp == 3:  # CALL FAR
                if seg is not None and off is not None:
                    new_ip = self.rw(seg, off)
                    new_cs = self.rw(seg, (off + 2) & 0xFFFF)
                    self.push(self.sregs['cs'])
                    self.push(self.ip)
                    self.sregs['cs'] = new_cs
                    self.ip = new_ip
            elif grp == 4:  # JMP r/m16
                self.ip = self.read_rm16(mod, rm, seg, off)
            elif grp == 5:  # JMP FAR
                if seg is not None and off is not None:
                    self.ip = self.rw(seg, off)
                    self.sregs['cs'] = self.rw(seg, (off + 2) & 0xFFFF)
            elif grp == 6:  # PUSH r/m16
                self.push(self.read_rm16(mod, rm, seg, off))

        # --- FE group ---
        elif op == 0xFE:
            mod, grp, rm, seg, off = self.decode_modrm(seg_override)
            v = self.read_rm8(mod, rm, seg, off)
            if grp == 0:
                r = (v + 1) & 0xFF
            else:
                r = (v - 1) & 0xFF
            self.write_rm8(mod, rm, seg, off, r)
            self.set_zf(r == 0)

        # --- JMP SHORT / NEAR / FAR ---
        elif op == 0xEB:
            disp = self.fetchs8()
            self.ip = (self.ip + disp) & 0xFFFF
        elif op == 0xE9:
            disp = self.fetchs16()
            self.ip = (self.ip + disp) & 0xFFFF
        elif op == 0xEA:
            new_ip = self.fetch16()
            self.sregs['cs'] = self.fetch16()
            self.ip = new_ip

        # --- CALL / RET ---
        elif op == 0xE8:
            rel = self.fetchs16()
            self.push(self.ip)
            self.ip = (self.ip + rel) & 0xFFFF
        elif op == 0xC3:
            self.ip = self.pop()
        elif op == 0xCB:
            self.ip = self.pop()
            self.sregs['cs'] = self.pop()
        elif op == 0xCF:  # IRET
            self.ip = self.pop()
            self.sregs['cs'] = self.pop()
            self.flags = self.pop()
        elif op == 0xC2:
            imm = self.fetch16()
            self.ip = self.pop()
            self.regs['sp'] = (self.regs['sp'] + imm) & 0xFFFF
        elif op == 0xCA:
            imm = self.fetch16()
            self.ip = self.pop()
            self.sregs['cs'] = self.pop()
            self.regs['sp'] = (self.regs['sp'] + imm) & 0xFFFF

        # --- ENTER / LEAVE (186+) ---
        elif op == 0xC8:
            size = self.fetch16()
            level = self.fetch8() & 0x1F
            self.push(self.regs['bp'])
            frame = self.regs['sp']
            if level > 0:
                for i in range(1, level):
                    self.regs['bp'] = (self.regs['bp'] - 2) & 0xFFFF
                    self.push(self.rw(self.sregs['ss'], self.regs['bp']))
                self.push(frame)
            self.regs['bp'] = frame
            self.regs['sp'] = (self.regs['sp'] - size) & 0xFFFF
        elif op == 0xC9:
            self.regs['sp'] = self.regs['bp']
            self.regs['bp'] = self.pop()

        # --- LOOP / LOOPNZ / LOOPZ / JCXZ ---
        elif op == 0xE2:
            disp = self.fetchs8()
            self.regs['cx'] = (self.regs['cx'] - 1) & 0xFFFF
            if self.regs['cx'] != 0:
                self.ip = (self.ip + disp) & 0xFFFF
        elif op == 0xE0:
            disp = self.fetchs8()
            self.regs['cx'] = (self.regs['cx'] - 1) & 0xFFFF
            if self.regs['cx'] != 0 and not self.get_zf():
                self.ip = (self.ip + disp) & 0xFFFF
        elif op == 0xE1:
            disp = self.fetchs8()
            self.regs['cx'] = (self.regs['cx'] - 1) & 0xFFFF
            if self.regs['cx'] != 0 and self.get_zf():
                self.ip = (self.ip + disp) & 0xFFFF
        elif op == 0xE3:
            disp = self.fetchs8()
            if self.regs['cx'] == 0:
                self.ip = (self.ip + disp) & 0xFFFF

        # --- Jcc short ---
        elif 0x70 <= op <= 0x7F:
            disp = self.fetchs8()
            zf = self.get_zf()
            cf = self.get_cf()
            sf = bool(self.flags & self.SF)
            of = bool(self.flags & self.OF)
            conds = {
                0: of, 1: not of, 2: cf, 3: not cf,
                4: zf, 5: not zf, 6: cf or zf, 7: not cf and not zf,
                8: sf, 9: not sf, 0xA: sf != of, 0xB: sf == of,
                0xC: zf or (sf != of), 0xD: not zf and (sf == of),
                0xE: zf or (sf != of), 0xF: not zf and (sf == of),
            }
            if conds[op - 0x70]:
                self.ip = (self.ip + disp) & 0xFFFF

        # --- String ops ---
        elif op == 0xAC:  # LODSB
            self.w8('al', self.rb(ds, self.regs['si']))
            self.regs['si'] = (self.regs['si'] + self.strd(1)) & 0xFFFF
        elif op == 0xAD:  # LODSW
            self.regs['ax'] = self.rw(ds, self.regs['si'])
            self.regs['si'] = (self.regs['si'] + self.strd(2)) & 0xFFFF
        elif op == 0xAA:  # STOSB
            self.wb(self.sregs['es'], self.regs['di'], self.r8('al'))
            self.regs['di'] = (self.regs['di'] + self.strd(1)) & 0xFFFF
        elif op == 0xAB:  # STOSW
            self.ww(self.sregs['es'], self.regs['di'], self.regs['ax'])
            self.regs['di'] = (self.regs['di'] + self.strd(2)) & 0xFFFF
        elif op == 0xA4:  # MOVSB
            self.wb(self.sregs['es'], self.regs['di'], self.rb(ds, self.regs['si']))
            self.regs['si'] = (self.regs['si'] + self.strd(1)) & 0xFFFF
            self.regs['di'] = (self.regs['di'] + self.strd(1)) & 0xFFFF
        elif op == 0xA5:  # MOVSW
            self.ww(self.sregs['es'], self.regs['di'], self.rw(ds, self.regs['si']))
            self.regs['si'] = (self.regs['si'] + self.strd(2)) & 0xFFFF
            self.regs['di'] = (self.regs['di'] + self.strd(2)) & 0xFFFF
        elif op == 0xA6:  # CMPSB
            a = self.rb(ds, self.regs['si'])
            b = self.rb(self.sregs['es'], self.regs['di'])
            self.update_flags_sub8(a, b, (a - b) & 0xFF)
            self.regs['si'] = (self.regs['si'] + self.strd(1)) & 0xFFFF
            self.regs['di'] = (self.regs['di'] + self.strd(1)) & 0xFFFF
        elif op == 0xA7:  # CMPSW
            a = self.rw(ds, self.regs['si'])
            b = self.rw(self.sregs['es'], self.regs['di'])
            self.update_flags_sub16(a, b, (a - b) & 0xFFFF)
            self.regs['si'] = (self.regs['si'] + self.strd(2)) & 0xFFFF
            self.regs['di'] = (self.regs['di'] + self.strd(2)) & 0xFFFF
        elif op == 0xAE:  # SCASB
            a = self.r8('al')
            b = self.rb(self.sregs['es'], self.regs['di'])
            self.update_flags_sub8(a, b, (a - b) & 0xFF)
            self.regs['di'] = (self.regs['di'] + self.strd(1)) & 0xFFFF
        elif op == 0xAF:  # SCASW
            a = self.regs['ax']
            b = self.rw(self.sregs['es'], self.regs['di'])
            self.update_flags_sub16(a, b, (a - b) & 0xFFFF)
            self.regs['di'] = (self.regs['di'] + self.strd(2)) & 0xFFFF

        # --- REP/REPNZ prefix ---
        elif op in (0xF2, 0xF3):
            repz = (op == 0xF3)
            nxt = self.fetch8()
            cx = self.regs['cx']
            d = self.strd(1)
            d2 = self.strd(2)
            es = self.sregs['es']

            if nxt == 0xAA:  # REP STOSB
                al = self.r8('al')
                di = self.regs['di']
                for _ in range(cx):
                    self.wb(es, di, al)
                    di = (di + d) & 0xFFFF
                self.regs['di'] = di
                self.regs['cx'] = 0
            elif nxt == 0xAB:  # REP STOSW
                ax = self.regs['ax']
                di = self.regs['di']
                for _ in range(cx):
                    self.ww(es, di, ax)
                    di = (di + d2) & 0xFFFF
                self.regs['di'] = di
                self.regs['cx'] = 0
            elif nxt == 0xA4:  # REP MOVSB
                si = self.regs['si']
                di = self.regs['di']
                for _ in range(cx):
                    self.wb(es, di, self.rb(ds, si))
                    si = (si + d) & 0xFFFF
                    di = (di + d) & 0xFFFF
                self.regs['si'] = si
                self.regs['di'] = di
                self.regs['cx'] = 0
            elif nxt == 0xA5:  # REP MOVSW
                si = self.regs['si']
                di = self.regs['di']
                for _ in range(cx):
                    self.ww(es, di, self.rw(ds, si))
                    si = (si + d2) & 0xFFFF
                    di = (di + d2) & 0xFFFF
                self.regs['si'] = si
                self.regs['di'] = di
                self.regs['cx'] = 0
            elif nxt == 0xAE:  # REPZ/REPNZ SCASB
                di = self.regs['di']
                al = self.r8('al')
                while cx > 0:
                    b = self.rb(es, di)
                    self.update_flags_sub8(al, b, (al - b) & 0xFF)
                    di = (di + d) & 0xFFFF
                    cx -= 1
                    if repz and not self.get_zf():
                        break
                    if not repz and self.get_zf():
                        break
                self.regs['di'] = di
                self.regs['cx'] = cx
            elif nxt == 0xAF:  # REPZ/REPNZ SCASW
                di = self.regs['di']
                ax = self.regs['ax']
                while cx > 0:
                    b = self.rw(es, di)
                    self.update_flags_sub16(ax, b, (ax - b) & 0xFFFF)
                    di = (di + d2) & 0xFFFF
                    cx -= 1
                    if repz and not self.get_zf():
                        break
                    if not repz and self.get_zf():
                        break
                self.regs['di'] = di
                self.regs['cx'] = cx
            elif nxt == 0xA6:  # REPZ/REPNZ CMPSB
                si = self.regs['si']
                di = self.regs['di']
                while cx > 0:
                    a = self.rb(ds, si)
                    b = self.rb(es, di)
                    self.update_flags_sub8(a, b, (a - b) & 0xFF)
                    si = (si + d) & 0xFFFF
                    di = (di + d) & 0xFFFF
                    cx -= 1
                    if repz and not self.get_zf():
                        break
                    if not repz and self.get_zf():
                        break
                self.regs['si'] = si
                self.regs['di'] = di
                self.regs['cx'] = cx
            elif nxt == 0xA7:  # REPZ/REPNZ CMPSW
                si = self.regs['si']
                di = self.regs['di']
                while cx > 0:
                    a = self.rw(ds, si)
                    b = self.rw(es, di)
                    self.update_flags_sub16(a, b, (a - b) & 0xFFFF)
                    si = (si + d2) & 0xFFFF
                    di = (di + d2) & 0xFFFF
                    cx -= 1
                    if repz and not self.get_zf():
                        break
                    if not repz and self.get_zf():
                        break
                self.regs['si'] = si
                self.regs['di'] = di
                self.regs['cx'] = cx
            elif nxt == 0xAC:  # REP LODSB
                si = self.regs['si']
                for _ in range(cx):
                    self.w8('al', self.rb(ds, si))
                    si = (si + d) & 0xFFFF
                self.regs['si'] = si
                self.regs['cx'] = 0
            elif nxt == 0xAD:  # REP LODSW
                si = self.regs['si']
                for _ in range(cx):
                    self.regs['ax'] = self.rw(ds, si)
                    si = (si + d2) & 0xFFFF
                self.regs['si'] = si
                self.regs['cx'] = 0
            else:
                raise ValueError(
                    f"Unhandled REP + 0x{nxt:02x} at "
                    f"{self.sregs['cs']:04x}:{(self.ip - 2) & 0xFFFF:04x}")

        # --- I/O (no-op in emulator) ---
        elif op in (0xEE, 0xEF):
            pass
        elif op in (0xEC, 0xED):
            self.w8('al', 0) if op == 0xEC else setattr(self.regs, 'ax', 0) or None
            if op == 0xED:
                self.regs['ax'] = 0
            else:
                self.w8('al', 0)
        elif op in (0xE4, 0xE5):
            self.fetch8()
            if op == 0xE4:
                self.w8('al', 0)
            else:
                self.regs['ax'] = 0
        elif op in (0xE6, 0xE7):
            self.fetch8()

        # --- INT ---
        elif op == 0xCC:
            pass  # INT 3
        elif op == 0xCD:
            self.fetch8()  # skip interrupt number
        elif op == 0xCE:
            pass  # INTO

        # --- XLAT ---
        elif op == 0xD7:
            self.w8('al', self.rb(ds, (self.regs['bx'] + self.r8('al')) & 0xFFFF))

        # --- FPU escape (D8-DF): skip ModR/M ---
        elif 0xD8 <= op <= 0xDF:
            self.decode_modrm(seg_override)

        # --- 0F two-byte opcodes (mostly NOPs or long Jcc for 286+) ---
        elif op == 0x0F:
            op2 = self.fetch8()
            if 0x80 <= op2 <= 0x8F:  # Jcc near (386+)
                disp = self.fetchs16()
                zf = self.get_zf()
                cf = self.get_cf()
                sf = bool(self.flags & self.SF)
                of = bool(self.flags & self.OF)
                cc = op2 & 0xF
                conds = {
                    0: of, 1: not of, 2: cf, 3: not cf,
                    4: zf, 5: not zf, 6: cf or zf, 7: not cf and not zf,
                    8: sf, 9: not sf, 0xA: sf != of, 0xB: sf == of,
                    0xC: zf or (sf != of), 0xD: not zf and (sf == of),
                    0xE: zf or (sf != of), 0xF: not zf and (sf == of),
                }
                if conds[cc]:
                    self.ip = (self.ip + disp) & 0xFFFF
            else:
                pass  # Unknown 0F xx — treat as NOP

        # --- BOUND (186+) ---
        elif op == 0x62:
            self.decode_modrm(seg_override)  # skip

        # --- ARPL (protected mode, NOP in real mode) ---
        elif op == 0x63:
            self.decode_modrm(seg_override)  # skip

        else:
            raise ValueError(
                f"Unknown opcode 0x{op:02x} at "
                f"{self.sregs['cs']:04x}:{(self.ip - 1) & 0xFFFF:04x}"
            )

        self.insn_count += 1
        return True

    def run(self, max_insns=50_000_000, stop_cs=None, stop_ip=None):
        """Run until stop condition or max instructions."""
        while self.insn_count < max_insns:
            if stop_cs is not None and self.sregs['cs'] == stop_cs and self.ip == stop_ip:
                return True
            if not self.step():
                return False
        return False


def load_exe(emu, exe_path, load_seg=0x1010):
    """Load a DOS MZ EXE into the emulator."""
    data = Path(exe_path).read_bytes()
    mz = struct.unpack('<16H', data[:32])
    assert mz[0] == 0x5A4D, "Not an MZ executable"

    hdr_size = mz[4] << 4
    image = data[hdr_size:]

    base = load_seg << 4
    for i, b in enumerate(image):
        if base + i < len(emu.mem):
            emu.mem[base + i] = b

    psp_seg = load_seg - 0x10
    emu.sregs['cs'] = load_seg + mz[0x0B]
    emu.ip = mz[0x0A]
    emu.sregs['ss'] = load_seg + mz[0x07]
    emu.regs['sp'] = mz[0x08]
    emu.sregs['ds'] = psp_seg
    emu.sregs['es'] = psp_seg

    return mz, data


def load_dos_memory(emu, dump_path, max_addr):
    """Load DOS memory dump into emulator, up to max_addr."""
    dos = Path(dump_path).read_bytes()
    n = min(len(dos), max_addr)
    emu.mem[:n] = dos[:n]
    print(f"Loaded DOS memory: {len(dos)} bytes from {dump_path}, "
          f"copied {n} bytes (up to 0x{max_addr:05x})")


def patch_encrypted_stub(emu, exe_data, cs_seg):
    """Replace encrypted stub with plaintext code from MZ header offset 0xC8.

    The plaintext at file offset 0xC8-0x1DF contains:
      CS:0000-001E  Anti-tamper hash check (XORs 8 words, expects 0x95B5)
      CS:001F-0045  Error handler (prints msg, enters EB FE infinite loop)
      CS:0046-00B3  Relocation fixer
      CS:00B4-00DF  Memory zeroing + far return to decompressed entry
      CS:00E0-0117  Data area with relocation tables

    Since patching changes the stub bytes, the hash check will fail.
    We patch the JZ at CS:001D (74 27) to JMP (EB 27) to skip the
    error handler unconditionally.
    """
    plaintext = bytearray(exe_data[0xC8:0x1E0])
    # Bypass anti-tamper: change JZ +0x27 at offset 0x1D to JMP +0x27
    if plaintext[0x1D] == 0x74:
        plaintext[0x1D] = 0xEB
        print(f"Bypassed anti-tamper hash check (JZ -> JMP at CS:001D)")
    base = cs_seg << 4
    emu.mem[base:base + len(plaintext)] = plaintext
    print(f"Patched {len(plaintext)} bytes of plaintext stub at "
          f"{cs_seg:04x}:0000 (phys 0x{base:05x})")


def verify_output(emu, ref_path, load_seg):
    """Compare emulator memory against a reference dump in the program area."""
    ref = Path(ref_path).read_bytes()
    base = load_seg << 4
    # Compare program area: from load_seg to end of reference or memory
    end = min(len(ref), len(emu.mem))
    mismatches = 0
    first_diff = None
    for i in range(base, end):
        if emu.mem[i] != ref[i]:
            mismatches += 1
            if first_diff is None:
                first_diff = i
    if mismatches == 0:
        print(f"Verification PASSED: program area matches {ref_path} "
              f"({end - base} bytes from 0x{base:05x})")
    else:
        print(f"Verification: {mismatches} mismatched bytes in range "
              f"0x{base:05x}-0x{end:05x}")
        if first_diff is not None:
            print(f"  First diff at phys 0x{first_diff:05x}: "
                  f"emu=0x{emu.mem[first_diff]:02x} ref=0x{ref[first_diff]:02x}")
            # Show a few more around the first diff
            for off in range(first_diff, min(first_diff + 16, end)):
                if emu.mem[off] != ref[off]:
                    seg = off >> 4
                    print(f"    0x{off:05x} ({seg:04x}:{off - (seg << 4):04x}): "
                          f"emu=0x{emu.mem[off]:02x} ref=0x{ref[off]:02x}")


def main():
    parser = argparse.ArgumentParser(
        description='8086 emulator for decrypting LZEXE-wrapped DOS executables')
    parser.add_argument('input_exe', help='DOS MZ executable to load')
    parser.add_argument('output', nargs='?', help='Memory dump output path')
    parser.add_argument('--dos-mem', metavar='PATH',
                        help='Load a DOS memory dump before loading the EXE')
    parser.add_argument('--patch-stub', action='store_true',
                        help='Replace encrypted stub with plaintext from MZ header')
    parser.add_argument('--verify', metavar='PATH',
                        help='Compare output against a reference memory dump')
    parser.add_argument('--max-insns', type=int, default=50_000_000,
                        help='Max instructions to execute (default: 50M)')
    args = parser.parse_args()

    exe_path = args.input_exe
    out_path = args.output

    emu = Emu8086()

    # Load DOS memory first (IVT, DOS kernel, etc.) if provided
    load_seg = 0x1010
    if args.dos_mem:
        load_dos_memory(emu, args.dos_mem, load_seg << 4)

    mz, exe_data = load_exe(emu, exe_path)

    init_cs = emu.sregs['cs']
    init_ip = emu.ip
    load_seg = init_cs - mz[0x0B]
    stub_seg = init_cs

    print(f"Loaded {exe_path}: CS={init_cs:04x}:{init_ip:04x}, "
          f"SS:SP={emu.sregs['ss']:04x}:{emu.regs['sp']:04x}, "
          f"load_seg={load_seg:04x}")

    if args.patch_stub:
        patch_encrypted_stub(emu, exe_data, stub_seg)

    # Run until CS changes (stub jumps to real program) or we hit an INT 21h
    prev_cs = init_cs
    try:
        while emu.insn_count < args.max_insns:
            cs = emu.sregs['cs']

            # Detect when CS changes — stub jumped to real code
            if cs != stub_seg:
                print(f"CS changed to {cs:04x}:{emu.ip:04x} "
                      f"after {emu.insn_count} insns — stub complete")
                break

            if emu.insn_count % 1_000_000 == 0 and emu.insn_count > 0:
                print(f"  ...{emu.insn_count} insns at "
                      f"{cs:04x}:{emu.ip:04x}")

            if not emu.step():
                print(f"Halted at {cs:04x}:{emu.ip:04x}")
                break

    except ValueError as e:
        print(f"Error after {emu.insn_count} insns: {e}")
        print(f"Regs: AX={emu.regs['ax']:04x} BX={emu.regs['bx']:04x} "
              f"CX={emu.regs['cx']:04x} DX={emu.regs['dx']:04x}")
        print(f"      SI={emu.regs['si']:04x} DI={emu.regs['di']:04x} "
              f"BP={emu.regs['bp']:04x} SP={emu.regs['sp']:04x}")
        print(f"      DS={emu.sregs['ds']:04x} ES={emu.sregs['es']:04x} "
              f"SS={emu.sregs['ss']:04x}")
        return 1

    print(f"Final: CS={emu.sregs['cs']:04x}:{emu.ip:04x} "
          f"SS:SP={emu.sregs['ss']:04x}:{emu.regs['sp']:04x} "
          f"DS={emu.sregs['ds']:04x} ES={emu.sregs['es']:04x} "
          f"({emu.insn_count} insns)")

    # Search for known strings in the decompressed memory
    mem = bytes(emu.mem)
    for sig in [b'Runtime error', b'Turbo Pascal', b'DATABASE',
                b'Esterian', b'.DAT', b'planet', b'fleet', b'Copyright',
                b'PLANETS.DAT', b'CHAIN.TXT', b'PLAYER.DAT', b'SETUP.DAT',
                b'ECGAME', b'ECMAINT']:
        pos = mem.find(sig, load_seg << 4)
        if pos >= 0:
            print(f"Found '{sig.decode()}' at phys 0x{pos:05x} "
                  f"(load+0x{pos - (load_seg << 4):05x})")

    if args.verify:
        verify_output(emu, args.verify, load_seg)

    if out_path:
        Path(out_path).write_bytes(mem)
        print(f"Wrote {len(mem)} bytes to {out_path}")

    return 0


if __name__ == '__main__':
    raise SystemExit(main())
