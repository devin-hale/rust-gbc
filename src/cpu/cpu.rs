use std::{
    ops::AddAssign,
    sync::{Arc, Mutex, MutexGuard},
};

use thiserror::Error;

use super::instr::{self, ADD, B3, Cond, DEC, INC, Instruction, LD, Mem, Operation, R8, R16, T3};
use crate::{
    cpu::instr::{Error as IError, JR, LDH, decode, decode_prefix},
    memory::{self, Memory},
    utils::bit,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Instruction error")]
    InstructionError(#[from] IError),

    #[error("CPU: unknown error")]
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InterruptControl {
    Enable,
    Disable,
}

pub struct CPU {
    a: Register,
    f: Register,
    b: Register,
    c: Register,
    d: Register,
    e: Register,
    h: Register,
    l: Register,
    sp: u16,
    pc: u16,

    mem: Arc<Mutex<Memory>>,

    stop: bool,
    halt: bool,

    prefix: bool,
    ic_0: Option<InterruptControl>,
    ic_1: Option<InterruptControl>,
    ime: bool,

    n16: Option<u16>,
    n8: Option<u8>,
}

#[derive(PartialEq, Eq)]
pub struct State {
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,

    mem: Vec<memory::State>,

    stop: bool,
    halt: bool,
    ie: bool,
    ime: bool,
}

#[derive(Clone, Copy)]
pub enum Flag {
    Z,
    N,
    H,
    C,
}

enum Interrupt {
    VBlank,
    LCD,
    STAT,
    Timer,
    Serial,
    Joypad,
}

struct Register(u8);

impl Register {
    fn new() -> Register {
        Register(0)
    }
    fn with_val(v: u8) -> Register {
        Register(v)
    }
    fn val(&self) -> u8 {
        self.0
    }

    fn write(&mut self, v: u8) {
        self.0 = v
    }

    fn inc(&mut self) -> u8 {
        self.0 += 1;
        self.0
    }

    fn dec(&mut self) -> u8 {
        self.0 -= 1;
        self.0
    }

    fn bit(&self, n: u8) -> u8 {
        bit::get(self.0, n)
    }

    fn bit_set(&mut self, n: u8) {
        bit::set(&mut self.0, n);
    }

    fn bit_reset(&mut self, n: u8) {
        bit::reset(&mut self.0, n);
    }
}

impl AddAssign<u8> for Register {
    fn add_assign(&mut self, rhs: u8) {
        self.write(rhs);
    }
}

impl AddAssign<u16> for Register {
    fn add_assign(&mut self, rhs: u16) {
        self.write((rhs & 0xFF) as u8);
    }
}

impl From<u8> for Register {
    fn from(value: u8) -> Self {
        Register(value)
    }
}

struct Word<'r> {
    high: &'r mut Register,
    low: &'r mut Register,
}

impl<'r> Word<'r> {
    fn new(high: &'r mut Register, low: &'r mut Register) -> Word<'r> {
        Word { high, low }
    }

    fn val(&self) -> u16 {
        ((self.high.0 as u16) << 8) | (self.low.0 as u16)
    }

    fn write(&mut self, w: u16) {
        self.low.0 = (w & 0xFF) as u8;
        self.high.0 = ((w & 0xFF00) >> 8) as u8;
    }

    fn inc(&mut self) -> u16 {
        self.write(self.val() + 1);
        self.val()
    }

    fn dec(&mut self) -> u16 {
        self.write(self.val() - 1);
        self.val()
    }
}

impl CPU {
    pub fn new(mem: &Arc<Mutex<Memory>>) -> CPU {
        CPU {
            a: Register::with_val(0x00),
            f: Register::with_val(0b1000_0000),
            b: Register::with_val(0x00),
            c: Register::with_val(0x13),
            d: Register::with_val(0x00),
            e: Register::with_val(0xD8),
            h: Register::with_val(0x01),
            l: Register::with_val(0x4D),
            sp: 0xFFFE,
            pc: 0x0100,
            mem: mem.clone(),
            prefix: false,
            stop: false,
            halt: false,
            ic_0: None,
            ic_1: None,
            ime: false,
            n8: None,
            n16: None,
        }
    }

    fn set_flag(&mut self, f: Flag) {
        match f {
            Flag::Z => self.f.bit_set(7),
            Flag::N => self.f.bit_set(6),
            Flag::H => self.f.bit_set(5),
            Flag::C => self.f.bit_set(4),
        }
    }

    fn reset_flag(&mut self, f: Flag) {
        match f {
            Flag::Z => self.f.bit_reset(7),
            Flag::N => self.f.bit_reset(6),
            Flag::H => self.f.bit_reset(5),
            Flag::C => self.f.bit_reset(4),
        }
    }

    fn set_flag_from_val(&mut self, f: Flag, v: u8) {
        if (v & 1) == 1 {
            self.set_flag(f);
        } else {
            self.reset_flag(f);
        }
    }

    fn invert_flag(&mut self, f: Flag) {
        if self.flag(f) {
            self.reset_flag(f);
        } else {
            self.set_flag(f);
        }
    }

    fn flag(&self, f: Flag) -> bool {
        let b = match f {
            Flag::Z => self.f.bit(7),
            Flag::N => self.f.bit(6),
            Flag::H => self.f.bit(5),
            Flag::C => self.f.bit(4),
        };
        b == 1
    }

    pub fn cf(&self) -> bool {
        self.flag(Flag::C)
    }
    pub fn hf(&self) -> bool {
        self.flag(Flag::H)
    }
    pub fn nf(&self) -> bool {
        self.flag(Flag::N)
    }
    pub fn zf(&self) -> bool {
        self.flag(Flag::Z)
    }

    fn clear_flags(&mut self) {
        self.reset_flag(Flag::Z);
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
        self.reset_flag(Flag::C);
    }

    fn af<'r>(&'r mut self) -> Word<'r> {
        Word::new(&mut self.a, &mut self.f)
    }

    fn hl<'r>(&'r mut self) -> Word<'r> {
        Word::new(&mut self.h, &mut self.l)
    }

    fn bc<'r>(&'r mut self) -> Word<'r> {
        Word::new(&mut self.b, &mut self.c)
    }

    fn de<'r>(&'r mut self) -> Word<'r> {
        Word::new(&mut self.d, &mut self.e)
    }

    fn mem<'m>(&'m mut self) -> MutexGuard<'m, Memory> {
        self.mem.lock().expect("error acquiring Memory mutex lock")
    }

    pub fn fetch(&mut self) -> u8 {
        let pc = self.pc;
        self.pc += 1;
        self.mem().read(pc)
    }

    pub fn imm(&mut self) -> u8 {
        let n = self.fetch();
        self.n8 = Some(n);
        n
    }

    pub fn fetch_word(&mut self) -> u16 {
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        (high << 8) | low
    }

    pub fn imm_word(&mut self) -> u16 {
        let n = self.fetch_word();
        self.n16 = Some(n);
        n
    }

    pub fn decode(&mut self, opcode: u8) -> Result<Instruction, Error> {
        if self.prefix {
            self.prefix = false;
            Ok(decode_prefix(opcode)?)
        } else {
            Ok(decode(opcode)?)
        }
    }

    pub fn execute(&mut self, i: Instruction) {
        let mut i = i;
        match i.op() {
            Operation::NOP => {}
            Operation::DI => self.di(),
            Operation::EI => self.ei(),
            Operation::STOP => self.stop = true,
            Operation::HALT => self.halt = true,
            Operation::LD(ld) => {
                self.ld(ld).unwrap();
            }
            Operation::SLA(r) => self.sla(r),
            Operation::SRA(r) => self.sra(r),
            Operation::SWAP(r) => self.swap(r),
            Operation::SRL(r) => self.srl(r),
            Operation::BIT(b, r) => self.bit(b, r),
            Operation::SET(b, r) => self.set(b, r),
            Operation::RES(b, r) => self.res(b, r),
            Operation::PREFIX => self.prefix = true,
            Operation::INC(inc) => self.inc(inc),
            Operation::DEC(dec) => self.dec(dec),
            Operation::ADD(add) => self.add(add),
            Operation::ADC(r) => self.adc(r),
            Operation::RLCA => self.rlc(R8::A),
            Operation::RLC(r) => self.rlc(r),
            Operation::RRCA => self.rrc(R8::A),
            Operation::RRC(r) => self.rrc(r),
            Operation::RLA => self.rl(R8::A),
            Operation::RL(r) => self.rl(r),
            Operation::RRA => self.rr(R8::A),
            Operation::RR(r) => self.rr(r),
            Operation::DAA => self.daa(),
            Operation::CPL => self.cpl(),
            Operation::SCF => self.scf(),
            Operation::CCF => self.ccf(),
            Operation::JP(r) => self.jp(r),
            Operation::JPC(c, r) => self.jp_cond(c, r),
            Operation::CALL => self.call(),
            Operation::CALLC(c) => self.call_cond(c),
            Operation::RST(t) => self.rst(t),
            Operation::POP(r) => self.pop(r),
            Operation::PUSH(r) => {
                let v = self.src_r16(r);
                self.push(v);
            }
            Operation::LDH(ldh) => match ldh {
                LDH::A(r) => self.ldh_a(r),
                LDH::Mem(r) => self.ldh_m(r),
            },
            Operation::JR(jr) => {
                let v = self.imm();
                match jr {
                    JR::Cond(c) => self.jr_cond(c, v),
                    JR::N8 => self.jr(v),
                }
            }
            Operation::RET => self.ret(),
            Operation::RETI => self.reti(),
            Operation::RETC(c) => self.ret_cond(c),
            Operation::SUB(r) => self.sub(r),
            Operation::SBC(r) => self.sbc(r),
            Operation::AND(r) => self.and(r),
            Operation::XOR(r) => self.xor(r),
            Operation::OR(r) => self.or(r),
            Operation::CP(r) => self.cp(r),
        }
        if let Some(n) = self.n8 {
            i.set_n8(n);
            self.n8 = None;
        }
        if let Some(n) = self.n16 {
            i.set_n16(n);
            self.n16 = None;
        }
    }

    pub fn cycle(&mut self) -> Result<(), Error> {
        if self.halt {
            // check for interrupt
            return Ok(());
        }
        if self.halt {
            // check for reset signal
            return Ok(());
        }
        self.handle_ic_0();
        let opcode = self.fetch();
        let i = self.decode(opcode)?;
        self.execute(i);
        self.handle_ic_1();
        Ok(())
    }

    fn jp(&mut self, r: R16) {
        let addr = self.src_r16(r);
        self.pc = addr;
    }

    fn jp_cond(&mut self, c: Cond, r: R16) {
        if self.cc(c) {
            self.jp(r)
        } else {
            self.fetch_word();
        }
    }

    fn jr(&mut self, b: u8) {
        self.pc += b as u16;
    }

    fn jr_cond(&mut self, c: Cond, b: u8) {
        if self.cc(c) {
            self.jr(b);
        } else {
            self.fetch();
        }
    }

    fn jr_word(&mut self, w: u16) {
        self.pc += w;
    }

    fn call(&mut self) {
        let pc = self.pc;
        self.push(pc);
        self.pc = self.fetch_word();
    }

    fn call_cond(&mut self, c: Cond) {
        if self.cc(c) {
            self.call();
        }
    }

    fn rst(&mut self, t: T3) {
        let pc = self.pc;
        let val = t.val();
        self.push(pc);
        self.pc = val as u16;
    }

    pub fn pop(&mut self, r: R16) {
        let sp = self.sp;
        self.sp += 2;
        let mut val = self.mem().read_word(sp);
        if r == R16::AF {
            val &= 0xFFF0;
        }
        self.ld_r16(r, val)
    }

    pub fn push(&mut self, v: u16) {
        self.sp -= 2;
        let sp = self.sp;
        self.mem().write_word(sp, v);
    }

    fn push_r16(&mut self, r: R16) {
        let val = self.src_r16(r);
        self.push(val);
    }

    fn cc(&self, c: Cond) -> bool {
        match c {
            Cond::Z => self.flag(Flag::Z),
            Cond::NZ => !self.flag(Flag::Z),
            Cond::C => self.flag(Flag::C),
            Cond::NC => !self.flag(Flag::C),
        }
    }

    fn ld(&mut self, op: LD) -> Result<(), ()> {
        match op {
            LD::R8(a, b) => {
                let v = self.src_r8(b);
                self.ld_r8(a, v);
            }
            LD::R16(a, b) => {
                let v = self.src_r16(b);
                self.ld_r16(a, v);
            }
            LD::MemR8(m, r) => {
                let addr = self.mem_addr(m);
                let v = self.src_r8(r);
                self.mem().write(addr, v);
            }
            LD::MemR16(m, r) => {
                let addr = self.mem_addr(m);
                let v = self.src_r16(r);
                self.mem().write_word(addr, v);
            }
            LD::R8Mem(r, m) => {
                let addr = self.mem_addr(m);
                let v = self.mem().read(addr);
                self.ld_r8(r, v);
            }
            LD::HLSPN => self.ld_hl_sp_n(),
        }
        Ok(())
    }

    pub fn mem_addr(&mut self, m: Mem) -> u16 {
        match m {
            Mem::HL => self.hl().val(),
            Mem::BC => self.bc().val(),
            Mem::DE => self.de().val(),
            Mem::SP => self.de().val(),
            Mem::SPN8 => {
                let n = self.imm();
                self.sp + (n as u16)
            }
            Mem::N16 => self.imm_word(),
            Mem::N8 => {
                let n = self.imm();
                return (n as u16) + 0xFF00;
            }
            Mem::C => (self.c.val() as u16) + 0xFF00,
            Mem::HLI => {
                let addr = self.hl().val();
                self.hl().inc();
                addr
            }
            Mem::HLD => {
                let addr = self.hl().val();
                self.hl().dec();
                addr
            }
        }
    }

    pub fn ld_r16(&mut self, r: R16, v: u16) {
        match r {
            R16::HL => self.hl().write(v),
            R16::BC => self.bc().write(v),
            R16::DE => self.de().write(v),
            R16::SP => self.sp = v,
            R16::AF => self.af().write(v),
            R16::N16 => panic!("attempt to load to n16 value"),

            _ => panic!("attempt to write {:b} to {}", v, r),
        }
    }

    pub fn ld_r8(&mut self, r: R8, v: u8) {
        match r {
            R8::A => self.a.write(v),
            R8::B => self.b.write(v),
            R8::C => self.c.write(v),
            R8::D => self.d.write(v),
            R8::E => self.e.write(v),
            R8::H => self.h.write(v),
            R8::L => self.l.write(v),
            R8::HL => {
                let addr = self.hl().val();
                self.mem().write(addr, v);
            }
            R8::N8 => panic!("attempt to load to n8 value"),
        }
    }

    fn ld_hl_sp_n(&mut self) {
        let sp = self.sp;
        let e = self.imm() as u16;
        let result = sp + e;
        self.hl().write(result);

        self.reset_flag(Flag::Z);
        self.reset_flag(Flag::N);
        if bit::check_overflow_word(sp, e, 3) {
            self.set_flag(Flag::H);
        }
        if bit::check_overflow_word(sp, e, 7) {
            self.set_flag(Flag::C);
        }
    }

    pub fn src_r8(&mut self, r: R8) -> u8 {
        match r {
            R8::N8 => self.imm(),
            R8::A => self.a.val(),
            R8::B => self.b.val(),
            R8::C => self.c.val(),
            R8::D => self.d.val(),
            R8::E => self.e.val(),
            R8::H => self.h.val(),
            R8::L => self.l.val(),
            R8::HL => {
                let addr = self.hl().val();
                return self.mem().read(addr);
            }
        }
    }

    pub fn src_r16(&mut self, r: R16) -> u16 {
        match r {
            R16::HL => self.hl().val(),
            R16::BC => self.bc().val(),
            R16::DE => self.de().val(),
            R16::PC => self.pc,
            R16::SP => self.sp,
            R16::AF => self.af().val(),
            R16::N16 => self.fetch_word(),
        }
    }

    fn inc(&mut self, i: INC) {
        match i {
            INC::R8(r) => self.inc_r8(r),
            INC::R16(r) => self.inc_r16(r),
        }
    }

    fn inc_r8(&mut self, r: R8) {
        let val = self.src_r8(r);
        let result = match r {
            R8::A | R8::B | R8::C | R8::D | R8::E | R8::H | R8::L => self.reg(r).inc(),
            R8::HL => {
                let addr = self.hl().val();
                self.mem().inc(addr)
            }
            _ => panic!("attempt to increment {}", r),
        };
        if result == 0 {
            self.reset_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        if bit::check_overflow(val, 1, 3) {
            self.set_flag(Flag::H);
        }
    }

    fn inc_r16(&mut self, r: R16) {
        match r {
            R16::DE => {
                self.de().inc();
            }
            R16::BC => {
                self.bc().inc();
            }
            R16::HL => {
                self.hl().inc();
            }
            R16::SP => {
                self.sp += 1;
            }
            R16::PC => {
                self.pc += 1;
            }
            _ => panic!("attempt to increment {}", r),
        }
    }

    fn dec(&mut self, d: DEC) {
        match d {
            DEC::R16(r) => self.dec_r16(r),
            DEC::R8(r) => self.dec_r8(r),
        }
    }

    fn dec_r8(&mut self, r: R8) {
        let val = self.src_r8(r);
        let result = match r {
            R8::A | R8::B | R8::C | R8::D | R8::E | R8::H | R8::L => self.reg(r).dec(),
            R8::HL => {
                let addr = self.hl().val();
                self.mem().dec(addr)
            }
            _ => panic!("attempt to increment {}", r),
        };

        if result == 0 {
            self.set_flag(Flag::Z);
        }
        if bit::check_borrow(val, 1, 4) {
            self.set_flag(Flag::H);
        }
        self.set_flag(Flag::N);
    }

    fn dec_r16(&mut self, r: R16) {
        match r {
            R16::DE | R16::BC | R16::HL => {
                self.reg_word(r).dec();
            }
            R16::SP => self.sp -= 1,
            R16::PC => self.pc -= 1,
            _ => panic!("attempt to decrement {}", r),
        }
    }

    fn reg(&mut self, r: R8) -> &mut Register {
        match r {
            R8::A => &mut self.a,
            R8::B => &mut self.b,
            R8::C => &mut self.c,
            R8::D => &mut self.d,
            R8::E => &mut self.e,
            R8::H => &mut self.h,
            R8::L => &mut self.l,
            R8::N8 => panic!("attempt to return n8 as 8 bit register"),
            R8::HL => panic!("attempt to return hl as 8 bit register"),
        }
    }

    fn reg_word<'a>(&'a mut self, r: R16) -> Word<'a> {
        match r {
            R16::DE => self.de(),
            R16::BC => self.bc(),
            R16::HL => self.hl(),
            _ => panic!("attempt to return {} as a 16 bit register", r),
        }
    }

    fn rlc(&mut self, r: R8) {
        let val = self.src_r8(r);
        let b7 = bit::get(val, 7);
        self.set_flag_from_val(Flag::C, b7);
        let result = (val << 1).wrapping_add(b7);
        self.ld_r8(r, result);

        if r == R8::A || result == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
    }

    fn rrc(&mut self, r: R8) {
        let val = self.src_r8(r);
        let b0 = bit::get(val, 0);
        self.set_flag_from_val(Flag::C, b0);
        let result = (val >> 1).wrapping_add(b0 << 7);
        self.ld_r8(r, result);

        if r == R8::A || result == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
    }

    fn rl(&mut self, r: R8) {
        let val = self.src_r8(r);
        let cf = self.cf() as u8;
        let b7 = bit::get(val, 7);
        self.set_flag_from_val(Flag::C, b7);
        let result = (val << 1).wrapping_add(cf);
        self.ld_r8(r, result);

        if r == R8::A || result == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
    }

    fn rr(&mut self, r: R8) {
        let val = self.src_r8(r);
        let cf = self.cf() as u8;
        let b0 = bit::get(val, 0);
        self.set_flag_from_val(Flag::C, b0);
        let result = (val >> 1).wrapping_add(cf << 7);
        self.ld_r8(r, result);

        if r == R8::A || result == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
    }

    fn ei(&mut self) {
        self.ic_0 = Some(InterruptControl::Enable);
        self.ic_1 = None;
    }

    fn di(&mut self) {
        self.ic_0 = Some(InterruptControl::Disable);
        self.ic_1 = None;
    }

    fn handle_ic_0(&mut self) {
        if let Some(ic) = self.ic_0 {
            self.ic_1 = Some(ic);
            self.ic_0 = None;
        }
    }

    fn handle_ic_1(&mut self) {
        if let Some(ic) = self.ic_1 {
            match ic {
                InterruptControl::Enable => self.ime = true,
                InterruptControl::Disable => self.ime = false,
            }
            self.ic_1 = None;
        }
    }

    fn daa(&mut self) {
        let a = self.a.val();
        if self.flag(Flag::N) {
            let mut adj = 0;
            if self.flag(Flag::H) {
                adj += 0x6;
            }
            if self.flag(Flag::C) {
                adj += 0x60;
            }
            let result = a - adj;
            self.a.write(result);

            self.reset_flag(Flag::H);
            if result == 0 {
                self.reset_flag(Flag::Z);
            }
        } else {
            let mut adj = 0;
            if self.flag(Flag::H) || (a & 0xF) > 0x9 {
                adj += 0x6;
            }
            if self.flag(Flag::C) || a > 0x99 {
                adj += 0x60;
                self.set_flag(Flag::C);
            }
            let result = a + adj;
            self.a.write(result);

            self.reset_flag(Flag::H);
            if result == 0 {
                self.reset_flag(Flag::Z);
            }
        }
    }

    fn cpl(&mut self) {
        let v = !self.a.val();
        self.a.write(v);
        self.set_flag(Flag::N);
        self.set_flag(Flag::H);
    }

    fn scf(&mut self) {
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
        self.set_flag(Flag::C);
    }

    fn ccf(&mut self) {
        self.reset_flag(Flag::N);
        self.set_flag_from_val(Flag::H, self.cf() as u8);
        self.invert_flag(Flag::C);
    }

    // ADD

    fn add(&mut self, add: ADD) {
        match add {
            ADD::A(r) => self.add_r8(r),
            ADD::HL(r) => self.add_r16(r),
            ADD::SP => self.add_sp(),
        }
    }

    fn add_r8(&mut self, r: R8) {
        let a = self.a.val();
        let val = self.src_r8(r);
        let result = a.wrapping_add(val);
        self.a.write(result);

        if result == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        if bit::check_overflow(a, val, 3) {
            self.set_flag(Flag::H);
        }
        if bit::check_overflow(a, val, 7) {
            self.set_flag(Flag::C);
        }
    }

    fn add_r16(&mut self, r: R16) {
        let hl = self.hl().val();
        let val = self.src_r16(r);
        let result = hl.wrapping_add(val);
        self.hl().write(result);

        self.reset_flag(Flag::N);
        if bit::check_overflow_word(hl, val, 11) {
            self.set_flag(Flag::H);
        }
        if bit::check_overflow_word(hl, val, 15) {
            self.set_flag(Flag::C);
        }
    }

    fn add_sp(&mut self) {
        let val = self.imm() as u16;
        let sp = self.sp;
        let result = sp.wrapping_add(val);
        self.sp = result;

        self.reset_flag(Flag::Z);
        self.reset_flag(Flag::N);
        if bit::check_overflow_word(sp, val, 3) {
            self.set_flag(Flag::H);
        }
        if bit::check_overflow_word(sp, val, 7) {
            self.set_flag(Flag::H);
        }
    }

    // ADC
    fn adc(&mut self, b: R8) {
        let a = self.a.val();
        let cf = self.cf() as u8;
        let val = self.src_r8(b);

        let result = a.wrapping_add(val.wrapping_add(cf));
        self.a.write(result);
        if result == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        if bit::check_overflow(a, val.wrapping_add(cf), 3) {
            self.set_flag(Flag::H);
        }
        if bit::check_overflow(a, val.wrapping_add(cf), 7) {
            self.set_flag(Flag::C);
        }
    }

    fn sub(&mut self, r: R8) {
        let a = self.a.val();
        let val = self.src_r8(r);
        let result = a.wrapping_sub(val);
        self.reg(r).write(result);

        if result == 0 {
            self.set_flag(Flag::Z);
        }
        self.set_flag(Flag::N);
        if bit::check_borrow(a, val, 4) {
            self.set_flag(Flag::H);
        }
        if bit::check_borrow(a, val, 8) {
            self.set_flag(Flag::C);
        }
    }

    fn sbc(&mut self, r: R8) {
        let a = self.a.val();
        let cf = self.cf() as u8;
        let val = self.src_r8(r);
        let result = a.wrapping_sub(val.wrapping_add(cf));
        self.a.write(result);

        if result == 0 {
            self.set_flag(Flag::Z);
        }
        self.set_flag(Flag::N);
        if bit::check_borrow(a, val, 4) {
            self.set_flag(Flag::H);
        }
        if bit::check_borrow(a, val + cf, 8) {
            self.set_flag(Flag::C);
        }
    }

    fn and(&mut self, r: R8) {
        let a = self.a.val();
        let val = self.src_r8(r);
        let result = a & val;
        self.a.write(result);

        if result == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.set_flag(Flag::H);
        self.reset_flag(Flag::C);
    }

    fn xor(&mut self, r: R8) {
        let a = self.a.val();
        let val = self.src_r8(r);
        let result = a ^ val;
        self.a.write(result);

        if result == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
        self.reset_flag(Flag::C);
    }

    fn or(&mut self, r: R8) {
        let a = self.a.val();
        let val = self.src_r8(r);
        let result = a | val;
        self.a.write(result);

        if result == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
        self.reset_flag(Flag::C);
    }

    fn cp(&mut self, r: R8) {
        let a = self.a.val();
        let val = self.src_r8(r);
        let result = a.wrapping_sub(val);

        if result == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        if bit::check_borrow(a, val, 4) {
            self.set_flag(Flag::H);
        }
        if bit::check_borrow(a, val, 8) {
            self.set_flag(Flag::C);
        }
    }

    fn ldh_a(&mut self, m: instr::Mem) {
        let addr = match m {
            instr::Mem::C => (self.c.val() as u16) + 0xFF00,
            instr::Mem::N8 => (self.imm() as u16) + 0xFF00,
            _ => panic!("invalid ldh destination operation"),
        };
        let val = self.mem().read(addr);
        self.a.write(val);
    }

    fn ldh_m(&mut self, m: instr::Mem) {
        let a = self.a.val();
        let addr = match m {
            instr::Mem::C => (self.c.val() as u16) | 0xFF00,
            instr::Mem::N8 => (self.imm() as u16) | 0xFF00,
            _ => panic!("invalid ldh destination operation"),
        };
        self.mem().write(addr, a);
    }

    fn ret(&mut self) {
        let addr = self.sp;
        let val = self.mem().read_word(addr);
        self.pc = val;
        self.sp += 2;
    }

    fn ret_cond(&mut self, c: Cond) {
        if self.cc(c) {
            self.ret();
        }
    }

    fn reti(&mut self) {
        self.ret();
        self.ime = true;
    }

    fn sla(&mut self, r: R8) {
        let v = self.src_r8(r);
        let v7 = bit::get(v, 7);
        self.set_flag_from_val(Flag::C, v7);
        let v = v << 1;
        if v == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
        self.ld_r8(r, v);
    }

    fn sra(&mut self, r: R8) {
        let v = self.src_r8(r);
        let v0 = bit::get(v, 0);
        let v7 = bit::get(v, 7) << 7;

        self.set_flag_from_val(Flag::C, v0);
        let v = v >> 1 | v7;
        if v == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
        self.ld_r8(r, v);
    }

    fn swap(&mut self, r: R8) {
        let v = self.src_r8(r);
        let l = (v & 0b1111) << 4;
        let h = (v & 0b1111_0000) >> 4;
        let v = l | h;
        self.ld_r8(r, v);
        if v == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
        self.reset_flag(Flag::C);
    }

    fn srl(&mut self, r: R8) {
        let v = self.src_r8(r);
        let v0 = bit::get(v, 0);
        self.set_flag_from_val(Flag::C, v0);
        let v = v >> 1;

        if v == 0 {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
        self.ld_r8(r, v);
    }

    fn bit(&mut self, b: B3, r: R8) {
        let val = self.src_r8(r);
        let i = b.val();

        if bit::is_set(val, i) {
            self.set_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.set_flag(Flag::H);
    }

    fn res(&mut self, b: B3, r: R8) {
        let mut val = self.src_r8(r);
        let i = b.val();
        bit::reset(&mut val, i);
        self.ld_r8(r, val);
    }

    fn set(&mut self, b: B3, r: R8) {
        let mut val = self.src_r8(r);
        let i = b.val();
        bit::set(&mut val, i);
        self.ld_r8(r, val);
    }

    fn query_interrupt(&mut self) -> Option<Interrupt> {
        let mut mem = self.mem();
        let iflags = mem.interrupt_flags();
        if iflags.vblank() {
            return Some(Interrupt::VBlank);
        }
        if iflags.lcd() {
            return Some(Interrupt::LCD);
        }
        if iflags.timer() {
            return Some(Interrupt::Timer);
        }
        if iflags.serial() {
            return Some(Interrupt::Serial);
        }
        if iflags.joypad() {
            return Some(Interrupt::Joypad);
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fetch() {
        let mem = Memory::arc();
        let val = 0xCC;
        let mut cpu = CPU::new(&mem);
        mem.lock().unwrap().write(cpu.pc, val);
        let pc = cpu.pc;
        let fetched = cpu.fetch();
        assert_eq!(val, fetched);
        assert_eq!(cpu.pc, pc + 1);
    }

    #[test]
    fn fetch_word() {
        let mem = Memory::arc();
        let mut cpu = CPU::new(&mem);
        let pc = cpu.pc;
        let low = 0xCC;
        let high = 0xDD;
        let word = 0xDDCC;
        mem.lock().unwrap().write(pc, low);
        mem.lock().unwrap().write(pc + 1, high);
        let fetched = cpu.fetch_word();
        assert_eq!(word, fetched);
        assert_eq!(cpu.pc, pc + 2);
    }

    #[test]
    fn jump_relative() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(&mem);
        let byte = 0xFF;
        let expected = cpu.pc + (byte as u16);
        cpu.jr(byte);
        assert_eq!(cpu.pc, expected);
    }

    #[test]
    fn jump_relative_word() {
        let mem = Memory::arc();
        let mut cpu = CPU::new(&mem);
        let word = 0x00FF;
        let expected = cpu.pc + word;
        cpu.jr_word(word);
        assert_eq!(cpu.pc, expected);
    }

    #[test]
    fn af() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(&mem);
        let byte = 0b0110_0110;
        let expected = ((byte as u16) << 8) | 0b1000_0000;
        cpu.set_flag(Flag::Z);
        cpu.a.write(byte);
        assert_eq!(cpu.af().val(), expected);
    }

    #[test]
    fn flag_set() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(&mem);
        cpu.set_flag(Flag::N);
        assert!(cpu.nf());
    }

    #[test]
    fn flag_reset() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(&mem);
        cpu.set_flag(Flag::N);
        assert!(cpu.nf());
        cpu.reset_flag(Flag::N);
        assert!(!cpu.nf());
    }

    #[test]
    fn flag() {
        let mem = Memory::arc();
        let mut cpu = CPU::new(&mem);
        assert!(!cpu.flag(Flag::N));
        cpu.set_flag(Flag::N);
        assert!(cpu.flag(Flag::N));
    }

    #[test]
    fn flag_set_val() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(&mem);
        let f = Flag::C;
        cpu.set_flag_from_val(f, 1);
        assert!(cpu.cf());
        cpu.set_flag_from_val(f, 0);
        assert!(!cpu.cf());
    }

    #[test]
    fn cc() {
        let mem = Memory::arc();
        let mut cpu = CPU::new(&mem);
        assert!(cpu.cc(Cond::Z));
        assert!(cpu.cc(Cond::NC));
        cpu.reset_flag(Flag::Z);
        cpu.set_flag(Flag::C);
        assert!(cpu.cc(Cond::NZ));
        assert!(cpu.cc(Cond::C));
    }

    #[test]
    fn register() {
        let byte = 0xFE;
        let mut r: Register = byte.into();
        assert_eq!(byte, r.val());
        assert_eq!(r.0, r.val());

        let byte = 0xF2;
        r.write(byte);
        assert_eq!(byte, r.val());
    }

    #[test]
    fn word_register() {
        let low_val = 0xFE;
        let high_val = 0xFE;
        let mut l: Register = low_val.into();
        let mut h: Register = high_val.into();

        let mut expected_word = ((high_val as u16) << 8) | (low_val as u16);

        let mut w = Word::new(&mut h, &mut l);
        assert_eq!(expected_word, w.val());

        expected_word += 1;
        w.inc();
        assert_eq!(expected_word, w.val());

        expected_word -= 1;
        w.dec();
        assert_eq!(expected_word, w.val());

        let low_val = 0xCC;
        let high_val = 0xDD;
        expected_word = ((high_val as u16) << 8) | (low_val as u16);

        w.write(expected_word);
        assert_eq!(expected_word, w.val());
        assert_eq!(l.0, low_val);
        assert_eq!(h.0, high_val);
    }

    fn setup() -> (CPU, Arc<Mutex<Memory>>) {
        let mem = Memory::arc();
        let cpu = CPU::new(&mem);
        (cpu, mem)
    }

    // LOAD INSTRUCTIONS
    #[test]
    fn ld_r_s() {
        for r in 0b000..=0b111 {
            for s in 0b000..=0b111 {
                let opcode = 0b0100_0000 | (r << 3) | s;
                let (mut cpu, _) = setup();
                let i = cpu.decode(opcode).unwrap();
                let a: R8 = r.try_into().unwrap();
                let b: R8 = s.try_into().unwrap();

                if a == R8::HL && b == R8::HL {
                    assert_eq!(i.op(), Operation::HALT);
                    continue;
                } else if a == R8::HL {
                    assert_eq!(i.op(), Operation::LD(LD::MemR8(Mem::HL, b)));
                } else if b == R8::HL {
                    assert_eq!(i.op(), Operation::LD(LD::R8Mem(a, Mem::HL)));
                } else {
                    assert_eq!(i.op(), Operation::LD(LD::R8(a, b)));
                }

                let val = 0xfe;
                cpu.ld_r8(b, val);
                cpu.execute(i);
                assert_eq!(cpu.src_r8(a), val);
            }
        }
        setup();
    }

    #[test]
    fn push() {
        for qq in 0..=3u8 {
            // 11qq0101
            let (mut cpu, mem) = setup();
            let opcode = 0b1100_0101 | (qq << 4);

            mem.lock().unwrap().write(cpu.pc, opcode);
            let fetched = cpu.fetch();
            assert_eq!(fetched, opcode);

            let r = R16::r16stk(qq).unwrap();
            let i = cpu.decode(opcode).unwrap();
            assert_eq!(i.op(), Operation::PUSH(r));

            let val = 0xfe;
            cpu.ld_r16(r, val);
            assert_eq!(cpu.src_r16(r), val);

            let sp = cpu.sp;
            cpu.execute(i);
            assert_eq!(mem.lock().unwrap().read_word(cpu.sp), val);
            assert_eq!(cpu.sp, sp - 2);
        }
    }

    #[test]
    fn pop() {
        for qq in 0..=3u8 {
            // 11qq0001
            let (mut cpu, mem) = setup();
            let opcode = 0b1100_0001 | (qq << 4);

            mem.lock().unwrap().write(cpu.pc, opcode);
            let fetched = cpu.fetch();
            assert_eq!(fetched, opcode);

            let r = R16::r16stk(qq).unwrap();
            let i = cpu.decode(opcode).unwrap();
            assert_eq!(i.op(), Operation::POP(r));

            let val = 0xDEAD;
            cpu.push(val);

            let sp = cpu.sp;
            cpu.execute(i);
            let r16 = cpu.src_r16(r);
            if r == R16::AF {
                assert_eq!(r16, val & 0xFFF0);
            } else {
                assert_eq!(r16, val);
            }
            assert_eq!(cpu.sp, sp + 2);
        }
    }

    #[test]
    fn ld_mem16() {
        for x in 0..=1 {
            for rr in 0..=3u8 {
                let (mut cpu, mem) = setup();
                // 0b00rr_x010
                let opcode = 0b0000_0010 | (rr << 4) | (x << 3);
                let r = Mem::r16mem(rr).unwrap();
                let i = cpu.decode(opcode).unwrap();
                if x == 1 {
                    assert_eq!(i.op(), Operation::LD(LD::R8Mem(R8::A, r)));

                    let val = 0xFE;
                    let addr;
                    if r == Mem::HLI || r == Mem::HLD {
                        addr = cpu.hl().val();
                    } else {
                        addr = cpu.mem_addr(r);
                    }
                    mem.lock().unwrap().write(addr, val);
                    assert_eq!(val, mem.lock().unwrap().read(addr));

                    cpu.execute(i);
                    assert_eq!(cpu.a.val(), val);
                } else {
                    assert_eq!(i.op(), Operation::LD(LD::MemR8(r, R8::A)));

                    let val = 0xFE;
                    cpu.a.write(val);
                    let addr;
                    if r == Mem::HLI || r == Mem::HLD {
                        addr = cpu.hl().val();
                    } else {
                        addr = cpu.mem_addr(r);
                    }
                    cpu.execute(i);
                    assert_eq!(mem.lock().unwrap().read(addr), val);
                }
            }
        }
    }

    #[test]
    fn ld_mem_n16_a() {
        let (mut cpu, mem) = setup();
        let opcode = 0b11101010;

        let addr = 0xDEAD;
        let val = 0xFE;
        mem.lock().unwrap().write_word(cpu.pc, addr);
        cpu.a.write(val);

        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::LD(LD::MemR8(Mem::N16, R8::A)));

        cpu.execute(i);
        assert_eq!(mem.lock().unwrap().read(addr), val);
    }

    #[test]
    fn ld_a_mem_n16() {
        let (mut cpu, mem) = setup();
        let opcode = 0b11111010;

        let addr = 0xDEAD;
        let val = 0xFE;
        mem.lock().unwrap().write_word(cpu.pc, addr);
        mem.lock().unwrap().write(addr, val);

        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::LD(LD::R8Mem(R8::A, Mem::N16)));

        cpu.execute(i);
        assert_eq!(cpu.a.val(), val);
    }

    #[test]
    fn ldh_n8_a() {
        for n in 0..u8::MAX {
            let (mut cpu, mem) = setup();
            let opcode = 0b11100000;

            let addr = n;
            let val = 0xFE;
            mem.lock().unwrap().write(cpu.pc, addr);
            cpu.a.write(val);

            let i = cpu.decode(opcode).unwrap();
            assert_eq!(i.op(), Operation::LDH(LDH::Mem(Mem::N8)));

            cpu.execute(i);
            assert_eq!(mem.lock().unwrap().read((addr as u16) | 0xFF00), val);
        }
    }

    #[test]
    fn ldh_a_n8() {
        let (mut cpu, mem) = setup();
        let opcode = 0b11110000;

        let addr = 0xDE;
        let val = 0xFE;
        mem.lock().unwrap().write(cpu.pc, addr);
        mem.lock().unwrap().write((addr as u16) | 0xFF00, val);

        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::LDH(LDH::A(Mem::N8)));

        cpu.execute(i);
        assert_eq!(cpu.a.val(), val);
    }

    #[test]
    fn ldh_memc_a() {
        let (mut cpu, mem) = setup();
        let opcode = 0b11100010;

        let addr = 0xDE;
        let val = 0xFE;
        cpu.c.write(addr);
        cpu.a.write(val);

        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::LDH(LDH::Mem(Mem::C)));

        cpu.execute(i);
        assert_eq!(mem.lock().unwrap().read((addr as u16) | 0xFF00), val);
    }

    #[test]
    fn ldh_a_memc() {
        let (mut cpu, mem) = setup();
        let opcode = 0b11110010;

        let addr = 0xDE;
        let val = 0xFE;
        cpu.c.write(addr);
        mem.lock().unwrap().write((addr as u16) | 0xFF00, val);

        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::LDH(LDH::A(Mem::C)));

        cpu.execute(i);
        assert_eq!(cpu.a.val(), val);
    }

    #[test]
    fn ld_pp_nn() {
        for pp in 0..=3u8 {
            let (mut cpu, mem) = setup();
            let val = 0xDEAD;
            mem.lock().unwrap().write_word(cpu.pc, val);

            let r: R16 = pp.try_into().unwrap();
            let opcode = 0b0000_0001 | (pp << 4);
            let i = cpu.decode(opcode).unwrap();
            assert_eq!(i.op(), Operation::LD(LD::R16(r, R16::N16)));

            cpu.execute(i);
            assert_eq!(cpu.src_r16(r), val);
        }
    }

    #[test]
    fn ld_mem_n16_sp() {
        let (mut cpu, mem) = setup();
        let opcode = 0b00001000;
        let word = 0xDEAD;
        mem.lock().unwrap().write_word(cpu.pc, word);
        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::LD(LD::MemR16(Mem::N16, R16::SP)));
        let sp = cpu.sp;
        cpu.execute(i);
        assert_eq!(sp, mem.lock().unwrap().read_word(word));
    }

    #[test]
    fn ld_sp_hl() {
        let (mut cpu, _) = setup();
        let opcode = 0b11111001;

        let word = 0xDEAD;
        cpu.hl().write(word);

        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::LD(LD::R16(R16::SP, R16::HL)));
        cpu.execute(i);

        assert_eq!(cpu.sp, word);
    }

    // 16 BIT ARITHMETIC INSTRUCTIONS

    #[test]
    fn inc_r16() {
        for pp in 0..=3u8 {
            let (mut cpu, _) = setup();
            let opcode = 0b0000_0011 | (pp << 4);
            let r: R16 = pp.try_into().unwrap();

            let i = cpu.decode(opcode).unwrap();
            assert_eq!(i.op(), Operation::INC(INC::R16(r)));
            let current = cpu.src_r16(r);
            cpu.execute(i);
            assert_eq!(cpu.src_r16(r), current + 1);
        }
    }

    #[test]
    fn dec_r16() {
        for pp in 0..=3u8 {
            let (mut cpu, _) = setup();
            let opcode = 0b0000_1011 | (pp << 4);
            let r: R16 = pp.try_into().unwrap();
            let val = 0xffee;
            cpu.ld_r16(r, val);
            let i = cpu.decode(opcode).unwrap();
            assert_eq!(i.op(), Operation::DEC(DEC::R16(r)));
            cpu.execute(i);
            assert_eq!(cpu.src_r16(r), val - 1);
        }
    }

    #[test]
    fn add_hl_r16() {
        for pp in 0..=3u8 {
            let (mut cpu, _) = setup();
            let opcode = 0b0000_1001 | (pp << 4);
            let r: R16 = pp.try_into().unwrap();
            let val = 0x0fee;
            cpu.ld_r16(r, val);
            let hl = cpu.hl().val();
            let i = cpu.decode(opcode).unwrap();
            assert_eq!(i.op(), Operation::ADD(ADD::HL(r)));
            cpu.execute(i);
            assert_eq!(cpu.hl().val(), hl + val);

            assert!(!cpu.flag(Flag::N));
            assert_eq!(cpu.flag(Flag::H), bit::check_overflow_word(hl, val, 11));
            assert_eq!(cpu.flag(Flag::C), bit::check_overflow_word(hl, val, 15));
        }
    }

    #[test]
    fn add_sp_e() {
        let (mut cpu, mem) = setup();
        let opcode = 0b11101000;
        let val = 0x11;
        cpu.sp = 0x00ff;
        mem.lock().unwrap().write(cpu.pc, val);
        let sp = cpu.sp;
        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::ADD(ADD::SP));
        cpu.execute(i);
        assert_eq!(cpu.sp, sp + (val as u16));

        assert!(!cpu.flag(Flag::Z));
        assert!(!cpu.flag(Flag::N));
        assert_eq!(
            cpu.flag(Flag::H),
            bit::check_overflow_word(sp, val as u16, 11)
        );
        assert_eq!(
            cpu.flag(Flag::C),
            bit::check_overflow_word(sp, val as u16, 15)
        );
    }

    #[test]
    fn ld_hl_spe() {
        let (mut cpu, mem) = setup();
        let opcode = 0b11111000;
        let val = 0x11;
        cpu.sp = 0x00ff;
        mem.lock().unwrap().write(cpu.pc, val);
        let sp = cpu.sp;
        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::LD(LD::HLSPN));
        cpu.execute(i);
        assert_eq!(cpu.hl().val(), sp + (val as u16));

        assert!(!cpu.flag(Flag::Z));
        assert!(!cpu.flag(Flag::N));
        assert_eq!(
            cpu.flag(Flag::H),
            bit::check_overflow_word(sp, val as u16, 11)
        );
        assert_eq!(
            cpu.flag(Flag::C),
            bit::check_overflow_word(sp, val as u16, 15)
        );
    }

    // 8 BIT ALU INSTRUCTIONS
    #[test]
    fn inc_r8() {
        for rrr in 0..=7u8 {
            let (mut cpu, _) = setup();
            let opcode = 0b00000100 | (rrr << 3);
            let r: R8 = rrr.try_into().unwrap();
            let i = cpu.decode(opcode).unwrap();
            assert_eq!(i.op(), Operation::INC(INC::R8(r)));
            let current = cpu.src_r8(r);
            cpu.execute(i);
            assert_eq!(cpu.src_r8(r), current + 1);

            if current + 1 == 0 {
                assert!(cpu.flag(Flag::Z));
            }
            assert!(!cpu.flag(Flag::N));
            if bit::check_overflow(current, 1, 3) {
                assert!(cpu.flag(Flag::Z));
            }
        }
    }

    #[test]
    fn dec_r8() {
        for rrr in 0..=7u8 {
            let (mut cpu, _) = setup();
            let opcode = 0b00000101 | (rrr << 3);
            let r: R8 = rrr.try_into().unwrap();
            let val = 0xde;
            cpu.ld_r8(r, val);

            let i = cpu.decode(opcode).unwrap();
            assert_eq!(i.op(), Operation::DEC(DEC::R8(r)));
            let current = cpu.src_r8(r);
            cpu.execute(i);
            assert_eq!(cpu.src_r8(r), current - 1);

            if current + 1 == 0 {
                assert!(cpu.flag(Flag::Z));
            }
            assert!(cpu.flag(Flag::N));
            if bit::check_borrow(current, 1, 4) {
                assert!(cpu.flag(Flag::H));
            }
        }
    }

    #[test]
    fn add_a_r8() {
        for rrr in 0..=7u8 {
            let (mut cpu, _) = setup();
            let opcode = 0b1000_0000 | rrr;
            let r: R8 = rrr.try_into().unwrap();

            let v1 = 0x3b;
            let v2 = 0x3b;

            cpu.a.write(v1);
            cpu.ld_r8(r, v2);

            let i = cpu.decode(opcode).unwrap();
            assert_eq!(i.op(), Operation::ADD(ADD::A(r)));

            cpu.execute(i);
            assert_eq!(cpu.a.val(), v1 + v2);

            if v1 + v2 == 0 {
                assert!(cpu.flag(Flag::Z));
            }
            assert!(!cpu.flag(Flag::N));
            if bit::check_overflow(v1, v2, 3) {
                assert!(cpu.flag(Flag::H));
            }
            if bit::check_overflow(v1, v2, 7) {
                assert!(cpu.flag(Flag::C));
            }
        }
    }

    #[test]
    fn add_a_n8() {
        for n in 0..=u8::MAX {
            let (mut cpu, mem) = setup();
            let opcode = 0b11000110;

            let v1 = 0x3b;

            cpu.a.write(v1);
            mem.lock().unwrap().write(cpu.pc, n);

            let i = cpu.decode(opcode).unwrap();
            assert_eq!(i.op(), Operation::ADD(ADD::A(R8::N8)));

            cpu.execute(i);
            assert_eq!(cpu.a.val(), v1.wrapping_add(n));

            if v1.wrapping_add(n) == 0 {
                assert!(cpu.flag(Flag::Z));
            }
            assert!(!cpu.flag(Flag::N));
            if bit::check_overflow(v1, n, 3) {
                assert!(cpu.flag(Flag::H));
            }
            if bit::check_overflow(v1, n, 7) {
                assert!(cpu.flag(Flag::C));
            }
        }
    }

    #[test]
    fn adc_a_r8() {
        for rrr in 0..=7u8 {
            let (mut cpu, _) = setup();
            let opcode = 0b1000_1000 | rrr;
            let r: R8 = rrr.try_into().unwrap();

            let v1 = 0x3b;
            let v2 = 0x3b;
            let cf = 0x01;

            cpu.set_flag(Flag::C);
            cpu.a.write(v1);
            cpu.ld_r8(r, v2);

            let i = cpu.decode(opcode).unwrap();
            assert_eq!(i.op(), Operation::ADC(r));

            cpu.execute(i);
            assert_eq!(cpu.a.val(), v1 + v2 + cf);

            if v1 + v2 + cf == 0 {
                assert!(cpu.flag(Flag::Z));
            }
            assert!(!cpu.flag(Flag::N));
            if bit::check_overflow(v1, v2 + cf, 3) {
                assert!(cpu.flag(Flag::H));
            }
            if bit::check_overflow(v1, v2 + cf, 7) {
                assert!(cpu.flag(Flag::C));
            }
        }
    }

    #[test]
    fn adc_a_n8() {
        let (mut cpu, mem) = setup();
        let opcode = 0b11001110;

        let v1 = 0x3b;
        let v2 = 0x3b;
        let cf = 1u8;

        cpu.set_flag(Flag::C);

        cpu.a.write(v1);
        mem.lock().unwrap().write(cpu.pc, v2);

        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::ADC(R8::N8));

        cpu.execute(i);
        assert_eq!(cpu.a.val(), v1 + v2 + cf);

        if v1 + v2 + cf == 0 {
            assert!(cpu.flag(Flag::Z));
        }
        assert!(!cpu.flag(Flag::N));
        if bit::check_overflow(v1, v2 + cf, 3) {
            assert!(cpu.flag(Flag::H));
        }
        if bit::check_overflow(v1, v2 + cf, 7) {
            assert!(cpu.flag(Flag::C));
        }
    }

    // ROT INSTRUCTIONS

    #[test]
    fn rlca() {
        for v in 0..=u8::MAX {
            let (mut cpu, _) = setup();
            let op = 0b0000_0111;
            cpu.a.write(v);

            let i = cpu.decode(op).unwrap();
            assert_eq!(i.op(), Operation::RLCA);
            cpu.execute(i);

            // cf := a.7, a := [a << 1] + cf
            let a = v;
            let a7 = bit::get(a, 7);
            let a = (a << 1) + a7;

            assert!(cpu.zf());
            assert!(!cpu.nf());
            assert!(!cpu.hf());
            assert_eq!(cpu.cf() as u8, a7);
            assert_eq!(cpu.a.val(), a);
        }
    }

    #[test]
    fn rrca() {
        for v in 0..=u8::MAX {
            let (mut cpu, _) = setup();
            let op = 0b0000_1111;
            cpu.a.write(v);

            let i = cpu.decode(op).unwrap();
            assert_eq!(i.op(), Operation::RRCA);
            cpu.execute(i);

            let a = v;
            let a0 = bit::get(a, 0);
            let a = (a >> 1) + (a0 << 7);

            assert!(cpu.zf());
            assert!(!cpu.nf());
            assert!(!cpu.hf());
            assert_eq!(cpu.cf() as u8, a0);
            assert_eq!(cpu.a.val(), a);
        }
    }

    #[test]
    fn rla() {
        for v in 0..=u8::MAX {
            let (mut cpu, _) = setup();
            let op = 0b0001_0111;
            cpu.a.write(v);

            let ocf = cpu.cf() as u8;
            let i = cpu.decode(op).unwrap();
            assert_eq!(i.op(), Operation::RLA);
            cpu.execute(i);

            let a = v;
            let a7 = bit::get(a, 7);
            let a = (a << 1) + ocf;

            assert!(cpu.zf());
            assert!(!cpu.nf());
            assert!(!cpu.hf());
            assert_eq!(cpu.cf() as u8, a7);
            assert_eq!(cpu.a.val(), a);
        }
    }

    #[test]
    fn rra() {
        for v in 0..=u8::MAX {
            let (mut cpu, _) = setup();
            let op = 0b0001_1111;
            cpu.a.write(v);

            let ocf = cpu.cf() as u8;
            let i = cpu.decode(op).unwrap();
            assert_eq!(i.op(), Operation::RRA);
            cpu.execute(i);

            let a = v;
            let a0 = bit::get(a, 0);
            let a = (a >> 1) + (ocf << 7);

            assert!(cpu.zf());
            assert!(!cpu.nf());
            assert!(!cpu.hf());
            assert_eq!(cpu.cf() as u8, a0);
            assert_eq!(cpu.a.val(), a);
        }
    }

    #[test]
    fn rr_r() {
        for rrr in 0..=7u8 {
            for v in 0..=u8::MAX {
                let (mut cpu, _) = setup();
                let prefix = cpu.decode(0xCB).unwrap();
                cpu.execute(prefix);

                let op = 0b0001_1000 | rrr;
                let r: R8 = rrr.try_into().unwrap();
                cpu.ld_r8(r, v);

                let ocf = cpu.cf() as u8;
                let i = cpu.decode(op).unwrap();
                assert_eq!(i.op(), Operation::RR(r));
                cpu.execute(i);

                let rv = v;
                let r0 = bit::get(rv, 0);
                let rv = (rv >> 1) + (ocf << 7);

                if rv == 0 {
                    assert!(cpu.zf());
                }
                assert!(!cpu.nf());
                assert!(!cpu.hf());
                assert_eq!(cpu.cf() as u8, r0);
                assert_eq!(cpu.src_r8(r), rv);
            }
        }
    }

    #[test]
    fn rl_r() {
        for rrr in 0..=7u8 {
            for v in 0..=u8::MAX {
                let (mut cpu, _) = setup();
                let prefix = cpu.decode(0xCB).unwrap();
                cpu.execute(prefix);

                let op = 0b0001_0000 | rrr;
                let r: R8 = rrr.try_into().unwrap();
                cpu.ld_r8(r, v);

                let ocf = cpu.cf() as u8;
                let i = cpu.decode(op).unwrap();
                assert_eq!(i.op(), Operation::RL(r));
                cpu.execute(i);

                let rv = v;
                let r7 = bit::get(rv, 7);
                let rv = (rv << 1) + ocf;

                if rv == 0 {
                    assert!(cpu.zf());
                }
                assert!(!cpu.nf());
                assert!(!cpu.hf());
                assert_eq!(cpu.cf() as u8, r7);
                assert_eq!(cpu.src_r8(r), rv);
            }
        }
    }

    #[test]
    fn rrc_r() {
        for rrr in 0..=7u8 {
            for v in 0..=u8::MAX {
                let (mut cpu, _) = setup();
                let prefix = cpu.decode(0xCB).unwrap();
                cpu.execute(prefix);

                let op = 0b00001000 | rrr;
                let r: R8 = rrr.try_into().unwrap();
                cpu.ld_r8(r, v);

                let i = cpu.decode(op).unwrap();
                assert_eq!(i.op(), Operation::RRC(r));
                cpu.execute(i);

                let rv = v;
                let r0 = bit::get(rv, 0);
                let rv = (rv >> 1) + (r0 << 7);

                if rv == 0 {
                    assert!(cpu.zf());
                }
                assert!(!cpu.nf());
                assert!(!cpu.hf());
                assert_eq!(cpu.cf() as u8, r0);
                assert_eq!(cpu.src_r8(r), rv);
            }
        }
    }

    #[test]
    fn rlc_r() {
        for rrr in 0..=7u8 {
            for v in 0..=u8::MAX {
                let (mut cpu, _) = setup();
                let prefix = cpu.decode(0xCB).unwrap();
                cpu.execute(prefix);

                let op = 0b00000000 | rrr;
                let r: R8 = rrr.try_into().unwrap();
                cpu.ld_r8(r, v);

                let i = cpu.decode(op).unwrap();
                assert_eq!(i.op(), Operation::RLC(r));
                cpu.execute(i);

                // cf := a.7, a := [a << 1] + cf
                let rv = v;
                let r7 = bit::get(rv, 7);
                let rv = (rv << 1) + r7;

                if rv == 0 {
                    assert!(cpu.zf());
                }
                assert!(!cpu.nf());
                assert!(!cpu.hf());
                assert_eq!(cpu.cf() as u8, r7);
                assert_eq!(cpu.src_r8(r), rv);
            }
        }
    }

    #[test]
    fn sla_r() {
        for rrr in 0..=7u8 {
            for v in 0..=u8::MAX {
                let (mut cpu, _) = setup();
                let prefix = cpu.decode(0xCB).unwrap();
                cpu.execute(prefix);

                let op = 0b0010_0000 | rrr;
                let r: R8 = rrr.try_into().unwrap();
                cpu.ld_r8(r, v);

                let i = cpu.decode(op).unwrap();
                assert_eq!(i.op(), Operation::SLA(r));
                cpu.execute(i);

                let rv = v;
                let r7 = bit::get(rv, 7);
                let rv = rv << 1;

                if rv == 0 {
                    assert!(cpu.zf());
                }
                assert!(!cpu.nf());
                assert!(!cpu.hf());

                assert_eq!(cpu.cf() as u8, r7);
                assert_eq!(cpu.src_r8(r), rv);
            }
        }
    }

    #[test]
    fn sra_r() {
        for rrr in 0..=7u8 {
            for v in 0..=u8::MAX {
                let (mut cpu, _) = setup();
                let prefix = cpu.decode(0xCB).unwrap();
                cpu.execute(prefix);

                let op = 0b0010_1000 | rrr;
                let r: R8 = rrr.try_into().unwrap();
                cpu.ld_r8(r, v);

                let i = cpu.decode(op).unwrap();
                assert_eq!(i.op(), Operation::SRA(r));
                cpu.execute(i);

                let rv = v;
                let r7 = bit::get(rv, 7) << 7;
                let r0 = bit::get(rv, 0);
                let rv = (rv >> 1) | r7;

                if rv == 0 {
                    assert!(cpu.zf());
                }
                assert!(!cpu.nf());
                assert!(!cpu.hf());

                assert_eq!(cpu.cf() as u8, r0);
                assert_eq!(cpu.src_r8(r), rv);
            }
        }
    }

    #[test]
    fn srl_r() {
        for rrr in 0..=7u8 {
            for v in 0..=u8::MAX {
                let (mut cpu, _) = setup();
                let prefix = cpu.decode(0xCB).unwrap();
                cpu.execute(prefix);

                let op = 0b0011_1000 | rrr;
                let r: R8 = rrr.try_into().unwrap();
                cpu.ld_r8(r, v);

                let i = cpu.decode(op).unwrap();
                assert_eq!(i.op(), Operation::SRL(r));
                cpu.execute(i);

                let rv = v;
                let r0 = bit::get(rv, 0);
                let rv = rv >> 1;

                if rv == 0 {
                    assert!(cpu.zf());
                }
                assert!(!cpu.nf());
                assert!(!cpu.hf());

                assert_eq!(cpu.cf() as u8, r0);
                assert_eq!(cpu.src_r8(r), rv);
            }
        }
    }

    #[test]
    fn swap() {
        for rrr in 0..=7u8 {
            for v in 0..=u8::MAX {
                let (mut cpu, _) = setup();
                let prefix = cpu.decode(0xCB).unwrap();
                cpu.execute(prefix);

                let op = 0b0011_0000 | rrr;
                let r: R8 = rrr.try_into().unwrap();
                cpu.ld_r8(r, v);

                let i = cpu.decode(op).unwrap();
                assert_eq!(i.op(), Operation::SWAP(r));
                cpu.execute(i);

                let high = (v & 0xF) << 4;
                let low = (v & 0xF0) >> 4;
                let rv = high | low;

                if rv == 0 {
                    assert!(cpu.zf());
                }
                assert!(!cpu.nf());
                assert!(!cpu.hf());
                assert!(!cpu.cf());

                assert_eq!(cpu.src_r8(r), rv);
            }
        }
    }

    // BITWISE INSTRUCTIONS

    #[test]
    fn bit_is_set() {
        for rrr in 0..=7u8 {
            for bbb in 0..=7u8 {
                for v in 0..=u8::MAX {
                    let (mut cpu, _) = setup();
                    let prefix = cpu.decode(0xCB).unwrap();
                    cpu.execute(prefix);

                    let op = 0b0100_0000 | (bbb << 3) | rrr;
                    let b: B3 = bbb.into();
                    let r: R8 = rrr.try_into().unwrap();
                    cpu.ld_r8(r, v);

                    let bit_set = bit::is_set(v, b.val());

                    let i = cpu.decode(op).unwrap();
                    assert_eq!(i.op(), Operation::BIT(b, r));
                    cpu.execute(i);

                    if bit_set {
                        assert!(cpu.zf());
                    }
                    assert!(!cpu.nf());
                    assert!(cpu.hf());
                }
            }
        }
    }

    #[test]
    fn bit_res() {
        for rrr in 0..=7u8 {
            for bbb in 0..=7u8 {
                for v in 0..=u8::MAX {
                    let (mut cpu, _) = setup();
                    let prefix = cpu.decode(0xCB).unwrap();
                    cpu.execute(prefix);

                    let op = 0b1000_0000 | (bbb << 3) | rrr;
                    let b: B3 = bbb.into();
                    let r: R8 = rrr.try_into().unwrap();
                    cpu.ld_r8(r, v);

                    let i = cpu.decode(op).unwrap();
                    assert_eq!(i.op(), Operation::RES(b, r));
                    cpu.execute(i);

                    assert!(!bit::is_set(cpu.src_r8(r), b.val()));
                }
            }
        }
    }

    #[test]
    fn bit_set() {
        for rrr in 0..=7u8 {
            for bbb in 0..=7u8 {
                for v in 0..=u8::MAX {
                    let (mut cpu, _) = setup();
                    let prefix = cpu.decode(0xCB).unwrap();
                    cpu.execute(prefix);

                    let op = 0b1100_0000 | (bbb << 3) | rrr;
                    let b: B3 = bbb.into();
                    let r: R8 = rrr.try_into().unwrap();
                    cpu.ld_r8(r, v);

                    let i = cpu.decode(op).unwrap();
                    assert_eq!(i.op(), Operation::SET(b, r));
                    cpu.execute(i);

                    assert!(bit::is_set(cpu.src_r8(r), b.val()));
                }
            }
        }
    }

    #[test]
    fn bit_cpl() {
        for v in 0..=u8::MAX {
            let (mut cpu, _) = setup();
            let op = 0b0010_1111;
            cpu.a.write(v);

            let i = cpu.decode(op).unwrap();
            assert_eq!(i.op(), Operation::CPL);
            cpu.execute(i);

            assert_eq!(!v, cpu.a.val());
            assert!(cpu.nf());
            assert!(cpu.hf());
        }
    }

    // CONTROL FLOW INSTRUCTIONS

    #[test]
    fn rst() {
        for ttt in 0..=7u8 {
            let (mut cpu, mem) = setup();
            let op = 0b1100_0111 | (ttt << 3);
            let t: T3 = ttt.into();
            let i = cpu.decode(op).unwrap();
            assert_eq!(i.op(), Operation::RST(t));

            let sp = cpu.sp;
            let pc = cpu.pc;

            cpu.execute(i);

            assert_eq!(mem.lock().unwrap().read_word(cpu.sp), pc);
            assert_eq!(cpu.sp, sp - 2);
            assert_eq!(cpu.pc as u8, t.val());
        }
    }

    #[test]
    fn call() {
        let (mut cpu, mem) = setup();
        let op = 0b11001101;
        let val = 0xDEAD;

        mem.lock().unwrap().write_word(cpu.pc, val);
        let i = cpu.decode(op).unwrap();
        assert_eq!(i.op(), Operation::CALL);

        let sp = cpu.sp;
        let pc = cpu.pc;

        cpu.execute(i);

        assert_eq!(cpu.sp, sp - 2);
        assert_eq!(mem.lock().unwrap().read_word(cpu.sp), pc);
        assert_eq!(cpu.pc, val);
    }

    #[test]
    fn call_cond() {
        // false
        for cc in 0..=3u8 {
            let (mut cpu, mem) = setup();
            let cond: Cond = cc.try_into().unwrap();
            let op = 0b11000100 | (cc << 3);
            let val = 0xDEAD;

            let cond_met = cpu.cc(cond);

            cpu.sp = 0xD00D;
            mem.lock().unwrap().write_word(cpu.pc, val);
            let i = cpu.decode(op).unwrap();
            assert_eq!(i.op(), Operation::CALLC(cond));

            let sp = cpu.sp;
            let pc = cpu.pc;

            cpu.execute(i);

            if cond_met {
                assert_eq!(cpu.sp, sp - 2);
                assert_eq!(mem.lock().unwrap().read_word(cpu.sp), pc);
                assert_eq!(cpu.pc, val);
            } else {
                assert_ne!(cpu.sp, sp - 2);
                assert_ne!(mem.lock().unwrap().read_word(cpu.sp), pc);
                assert_ne!(cpu.pc, val);
            }
        }
    }

    #[test]
    fn jp() {
        let (mut cpu, mem) = setup();
        let val = 0xDEAD;
        let op = 0b11000011;

        mem.lock().unwrap().write_word(cpu.pc, val);
        let i = cpu.decode(op).unwrap();
        assert_eq!(i.op(), Operation::JP(R16::N16));

        cpu.execute(i);
        assert_eq!(cpu.pc, val);
    }

    #[test]
    fn jp_hl() {
        let (mut cpu, _) = setup();
        let val = 0xDEAD;
        let op = 0b11101001;
        cpu.hl().write(val);
        let i = cpu.decode(op).unwrap();
        assert_eq!(i.op(), Operation::JP(R16::HL));
        cpu.execute(i);
        assert_eq!(cpu.pc, val);
    }

    #[test]
    fn jp_cond() {
        for cc in 0..=3u8 {
            let (mut cpu, mem) = setup();
            let cond: Cond = cc.try_into().unwrap();
            let op = 0b1100_0010 | (cc << 3);
            let val = 0xDEAD;

            mem.lock().unwrap().write_word(cpu.pc, val);
            let i = cpu.decode(op).unwrap();
            assert_eq!(i.op(), Operation::JPC(cond, R16::N16));

            cpu.execute(i);

            if cpu.cc(cond) {
                assert_eq!(cpu.pc, val);
            }
        }
    }

    #[test]
    fn jr() {
        let (mut cpu, mem) = setup();
        let op = 0b00011000;
        let val = 0x1E;
        mem.lock().unwrap().write_word(cpu.pc, val);

        let pc = cpu.pc;

        let i = cpu.decode(op).unwrap();
        assert_eq!(i.op(), Operation::JR(JR::N8));
        cpu.execute(i);

        assert_eq!(cpu.pc, pc + 1 + val);
    }

    #[test]
    fn jr_cond() {
        for cc in 0..=3u8 {
            let (mut cpu, mem) = setup();
            let op = 0b0010_0000 | (cc << 3);
            let cond: Cond = cc.try_into().unwrap();
            let val = 0x1E;
            mem.lock().unwrap().write_word(cpu.pc, val);
            let pc = cpu.pc;
            let i = cpu.decode(op).unwrap();
            assert_eq!(i.op(), Operation::JR(JR::Cond(cond)));
            cpu.execute(i);
            if cpu.cc(cond) {
                assert_eq!(cpu.pc, pc + 1 + val);
            }
        }
    }

    #[test]
    fn ret() {
        let (mut cpu, mem) = setup();
        let op = 0b1100_1001;
        let addr = 0xeeee;
        cpu.sp = addr;
        let sp = cpu.sp;
        let val = 0xDEAD;
        mem.lock().unwrap().write_word(addr, val);

        let i = cpu.decode(op).unwrap();
        assert_eq!(i.op(), Operation::RET);
        cpu.execute(i);

        assert_eq!(cpu.pc, mem.lock().unwrap().read_word(addr));
        assert_eq!(val, mem.lock().unwrap().read_word(addr));
        assert_eq!(cpu.sp, sp + 2);
    }

    #[test]
    fn ret_cond() {
        for cc in 0..=3u8 {
            let cond: Cond = cc.try_into().unwrap();
            let (mut cpu, mem) = setup();
            let op = 0b1100_0000 | (cc << 3);
            let addr = 0xeeee;
            cpu.sp = addr;
            let sp = cpu.sp;
            let val = 0xDEAD;
            mem.lock().unwrap().write_word(addr, val);

            let i = cpu.decode(op).unwrap();
            assert_eq!(i.op(), Operation::RETC(cond));
            cpu.execute(i);

            if cpu.cc(cond) {
                assert_eq!(cpu.pc, mem.lock().unwrap().read_word(addr));
                assert_eq!(cpu.pc, val);
                assert_eq!(cpu.sp, sp + 2);
            } else {
                assert_ne!(cpu.pc, mem.lock().unwrap().read_word(addr));
                assert_ne!(cpu.pc, val);
                assert_ne!(cpu.sp, sp + 2);
            }
        }
    }

    #[test]
    fn reti() {
        let (mut cpu, mem) = setup();
        let op = 0b1101_1001;
        let addr = 0xeeee;
        cpu.sp = addr;
        let sp = cpu.sp;
        let val = 0xDEAD;
        mem.lock().unwrap().write_word(addr, val);

        let i = cpu.decode(op).unwrap();
        assert_eq!(i.op(), Operation::RETI);
        cpu.execute(i);

        assert_eq!(cpu.pc, mem.lock().unwrap().read_word(addr));
        assert_eq!(val, mem.lock().unwrap().read_word(addr));
        assert_eq!(cpu.sp, sp + 2);
        assert!(cpu.ime);
    }

    // CPU CONTROL INSTRUCTIONS

    #[test]
    fn di() {
        let (mut cpu, mem) = setup();
        cpu.ime = true;
        let opcode = 0b11110011;
        mem.lock().unwrap().write(cpu.pc, opcode);
        cpu.cycle().unwrap();

        assert!(cpu.ime);

        cpu.cycle().unwrap();
        assert!(!cpu.ime);
    }

    #[test]
    fn ei() {
        let (mut cpu, mem) = setup();
        let opcode = 0b11111011;
        mem.lock().unwrap().write(cpu.pc, opcode);
        cpu.cycle().unwrap();

        assert!(!cpu.ime);

        cpu.cycle().unwrap();
        assert!(cpu.ime);
    }

    #[test]
    fn stop() {
        let opcode = 0b00010000;
        let (mut cpu, _) = setup();
        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::STOP);
        cpu.execute(i);
        assert!(cpu.stop);
    }

    #[test]
    fn halt() {
        let opcode = 0b0111_0110;
        let (mut cpu, _) = setup();
        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::HALT);
        cpu.execute(i);
        assert!(cpu.halt);
    }

    #[test]
    fn scf() {
        let op = 0b00110111;
        let (mut cpu, _) = setup();
        let i = cpu.decode(op).unwrap();
        assert_eq!(i.op(), Operation::SCF);
        cpu.execute(i);
        assert!(!cpu.flag(Flag::N));
        assert!(!cpu.flag(Flag::H));
        assert!(cpu.flag(Flag::C));
    }

    #[test]
    fn ccf() {
        let op = 0b00111111;
        let (mut cpu, _) = setup();
        let i = cpu.decode(op).unwrap();
        assert_eq!(i.op(), Operation::CCF);
        let cf = cpu.cf();

        cpu.execute(i);
        assert!(!cpu.flag(Flag::N));
        assert_eq!(cpu.flag(Flag::H), cf);
        assert_eq!(cpu.flag(Flag::C), !cf);
    }
}
