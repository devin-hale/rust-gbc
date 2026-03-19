use std::{
    ops::AddAssign,
    sync::{Arc, Mutex, MutexGuard},
};

pub const MASTER_CLOCK_FREQ: u64 = 4_194_304; // Hz
pub const T_CYCLE_PRD_NS: u64 = (1 * 1000 * 1000 * 1000) / MASTER_CLOCK_FREQ; // ns
pub const DIV_CLOCK_FREQ: u64 = 16_384; // Hz
pub const DIV_CLOCK_PRD_NS: u64 = (1 * 1000 * 1000 * 1000) / DIV_CLOCK_FREQ; // ns
const CYCLES_PER_DIV_TICK: u64 = DIV_CLOCK_PRD_NS / T_CYCLE_PRD_NS;
pub const SYSTEM_CLOCK: u64 = MASTER_CLOCK_FREQ / 4;

use serde::{
    Deserialize, Deserializer,
    de::{self, Visitor},
};
use serde_json::Value;
use thiserror::Error;

use super::instr::{self, ADD, B3, Cond, DEC, INC, Instruction, LD, Mem, R8, R16, T3};
use crate::{
    cpu::instr::{
        ADC, Add, Dec, Error as IError, Fetch, Inc, JR, LDH, Load, Op, SBC, Sub, decode,
        decode_prefix,
    },
    memory::{self, AddressBus, DataBus, Memory},
    utils::bit,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Instruction error")]
    InstructionError(#[from] IError),

    #[error("CPU: unknown error")]
    Unknown,
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

    addr_bus: AddressBus,
    data_bus: DataBus,

    ir: Instruction,
    stop: bool,
    halt: bool,

    //prefix: bool,
    //ic_0: Option<InterruptControl>,
    //ic_1: Option<InterruptControl>,
    ime: bool,
    ie: bool,
    //div_cycles: u64,
    //timer_cycles: u64,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct State {
    pc: u16,
    sp: u16,
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    ie: Option<u8>,
    ime: u8,
    ram: Vec<[u16; 2]>,
}

#[derive(Debug)]
struct CycleState {
    addr: Option<u16>,
    data: Option<u8>,
    r: bool,
    w: bool,
    m: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Flag {
    Z,
    N,
    H,
    C,
}

#[derive(Debug, Deserialize)]
struct Test {
    name: String,
    initial: State,
    r#final: State,
    cycles: Vec<CycleState>,
}

impl<'de> de::Deserialize<'de> for CycleState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CycleStateVisitor;

        impl<'de> Visitor<'de> for CycleStateVisitor {
            type Value = CycleState;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("CycleState")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut cs: CycleState = CycleState {
                    addr: None,
                    data: None,
                    r: false,
                    w: false,
                    m: false,
                };
                let mut index = 0;
                while let Some(el) = seq.next_element()? {
                    match index {
                        0 => {
                            if let Value::Number(n) = el {
                                match n.as_u64() {
                                    Some(addr) => {
                                        cs.addr = Some(addr as u16);
                                    }
                                    None => {
                                        return Err(de::Error::custom(format!(
                                            "invalid value for CycleState address: {:?}",
                                            n
                                        )));
                                    }
                                }
                            } else if let Value::Null = el {
                                cs.addr = None;
                            } else {
                                return Err(de::Error::custom(
                                    "invalid value for CycleState address",
                                ));
                            };
                        }
                        1 => {
                            if let Value::Number(n) = el {
                                match n.as_u64() {
                                    Some(data) => {
                                        cs.data = Some(data as u8);
                                    }
                                    None => {
                                        return Err(de::Error::custom(format!(
                                            "invalid value for CycleState data: {:?}",
                                            n
                                        )));
                                    }
                                }
                            } else if let Value::Null = el {
                                cs.data = None;
                            } else {
                                return Err(de::Error::custom("invalid value for CycleState data"));
                            };
                        }
                        2 => {
                            let Value::String(s) = el else {
                                return Err(de::Error::custom(
                                    "invalid value for CycleState rwm pins",
                                ));
                            };
                            if s.len() != 3 {
                                return Err(de::Error::custom(format!(
                                    "Invalid value for CycleState rwm: {}",
                                    s
                                )));
                            }
                            match s.chars().nth(0) {
                                Some(v) => {
                                    cs.r = v == 'r';
                                }
                                None => {
                                    return Err(de::Error::custom("no flag for 'r'"));
                                }
                            };
                            match s.chars().nth(1) {
                                Some(v) => {
                                    cs.w = v == 'w';
                                }
                                None => {
                                    return Err(de::Error::custom("no flag for 'w'"));
                                }
                            };
                            match s.chars().nth(2) {
                                Some(v) => {
                                    cs.m = v == 'm';
                                }
                                None => {
                                    return Err(de::Error::custom("no flag for 'm'"));
                                }
                            };
                        }
                        _ => return Err(de::Error::custom("invalid value for CycleState")),
                    }
                    index += 1;
                }
                Ok(cs)
            }
        }

        let visitor = CycleStateVisitor;
        deserializer.deserialize_seq(visitor)
    }
}

#[derive(Clone, Copy)]
pub enum Interrupt {
    VBlank,
    STAT,
    Timer,
    Serial,
    Joypad,
}

impl Interrupt {
    fn addr(&self) -> u8 {
        match self {
            Interrupt::VBlank => 0x40,
            Interrupt::STAT => 0x48,
            Interrupt::Timer => 0x50,
            Interrupt::Serial => 0x58,
            Interrupt::Joypad => 0x60,
        }
    }
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
        self.0 = self.0.wrapping_add(1);
        self.0
    }

    fn dec(&mut self) -> u8 {
        self.0 = self.0.wrapping_sub(1);
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
    pub fn new(mem: &Memory) -> CPU {
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
            addr_bus: mem.address_bus(memory::Accessor::CPU),
            data_bus: mem.data_bus(memory::Accessor::CPU),
            ir: Instruction::nop(),
            //prefix: false,
            stop: false,
            halt: false,
            //ic_0: None,
            //ic_1: None,
            ie: false,
            ime: false,
            //n8: None,
            //n16: None,
            //div_cycles: 0,
            //timer_cycles: 0,
        }
    }

    fn state(&self) -> State {
        State {
            pc: self.pc,
            sp: self.sp,
            a: self.a.0,
            f: self.f.0,
            b: self.b.0,
            c: self.c.0,
            d: self.d.0,
            e: self.e.0,
            h: self.h.0,
            l: self.l.0,
            ie: Some(self.ie as u8),
            ime: self.ime as u8,
            ram: vec![],
        }
    }

    fn load_state(&mut self, s: &State) {
        self.pc = s.pc;
        self.sp = s.sp;
        self.a.0 = s.a;
        self.f.0 = s.f;
        self.b.0 = s.b;
        self.c.0 = s.c;
        self.d.0 = s.d;
        self.e.0 = s.e;
        self.h.0 = s.h;
        self.l.0 = s.l;
        if let Some(ie) = s.ie {
            match ie {
                0 => self.ie = false,
                _ => self.ie = true,
            }
        }
        match s.ime {
            0 => self.ime = false,
            _ => self.ime = true,
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

    fn cf(&self) -> bool {
        self.flag(Flag::C)
    }
    fn hf(&self) -> bool {
        self.flag(Flag::H)
    }
    fn nf(&self) -> bool {
        self.flag(Flag::N)
    }
    fn zf(&self) -> bool {
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

    // marks first
    fn prime_test(&mut self) {}

    fn fetch(&mut self) -> u8 {
        self.addr_bus.assert(self.pc);
        self.pc = self.pc.wrapping_add(1);
        self.data_bus.read()
    }

    //fn imm(&mut self) -> u8 {
    //    let n = self.fetch();
    //    self.n8 = Some(n);
    //    n
    //}

    fn fetch_word(&mut self) -> u16 {
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        (high << 8) | low
    }

    //fn imm_word(&mut self) -> u16 {
    //    let n = self.fetch_word();
    //    self.n16 = Some(n);
    //    n
    //}

    //fn decode(&mut self, opcode: u8) -> Result<Instruction, Error> {
    //    if self.prefix {
    //        self.prefix = false;
    //        Ok(decode_prefix(opcode)?)
    //    } else {
    //        Ok(decode(opcode)?)
    //    }
    //}

    pub fn execute(&mut self) -> Result<(), Error> {
        if self.ir.done() {
            return Ok(());
        }
        let mut ex = vec![];
        for s in self.ir.steps_mut() {
            if !s.is_done() {
                for op in s.ops().iter() {
                    ex.push(*op);
                }
                s.set_done();
                break;
            }
        }
        if ex.len() == 0 {
            self.ir.complete();
        } else {
            for op in ex {
                self.execute_op(op)?;
                if let Op::CheckCond(_) = op
                    && self.ir.done()
                {
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    fn execute_op(&mut self, op: Op) -> Result<(), Error> {
        match op {
            Op::Stop => self.handle_stop(),
            Op::Halt => self.handle_halt(),
            Op::Add(a) => self.handle_add(a),
            Op::ADC(a) => self.handle_adc(a),
            Op::Sub(s) => self.handle_sub(s),
            Op::SBC(s) => self.handle_sbc(s),
            Op::AND(r) => self.handle_and(r),
            Op::XOR(r) => self.handle_xor(r),
            Op::OR(r) => self.handle_or(r),
            Op::Fetch(f) => self.handle_fetch(f)?,
            Op::Load(l) => self.handle_load(l)?,
            Op::Assert(r) => self.handle_assert(r),
            Op::AssertInc(r) => self.handle_assert_inc(r),
            Op::Inc(i) => self.handle_inc(i),
            Op::Dec(d) => self.handle_dec(d),
            Op::RLC(r) => self.handle_rlc(r),
            Op::RL(r) => self.handle_rl(r),
            Op::RRC(r) => self.handle_rrc(r),
            Op::RR(r) => self.handle_rr(r),
            Op::DAA => self.handle_daa(),
            Op::SCF => self.handle_scf(),
            Op::CPL => self.handle_cpl(),
            Op::CCF => self.handle_ccf(),
            Op::CheckCond(c) => self.handle_check_cond(c),
            _ => todo!("op {:?}", op),
        }
        Ok(())
    }

    fn handle_fetch(&mut self, f: Fetch) -> Result<(), Error> {
        match f {
            Fetch::NnLo => {
                let val = self.fetch();
                self.ir.set_n16_lo(val);
            }
            Fetch::NnHi => {
                let val = self.fetch();
                self.ir.set_n16_hi(val);
            }
            Fetch::N => {
                let val = self.fetch();
                self.ir.set_n8(val);
            }
            Fetch::E => {
                let val = self.fetch() as i8;
                self.ir.set_e(val);
            }
        }
        Ok(())
    }

    fn handle_halt(&mut self) {
        self.halt = true;
    }

    fn handle_inc(&mut self, i: Inc) {
        match i {
            Inc::Register(r) => self.handle_inc_register(r),
            Inc::Memory => self.handle_inc_mem(),
        }
    }

    fn handle_dec(&mut self, d: Dec) {
        match d {
            Dec::Register(r) => self.handle_dec_register(r),
            Dec::Memory => self.handle_dec_mem(),
        }
    }

    fn handle_inc_mem(&mut self) {
        let val = self.data_bus.read();
        let result = val.wrapping_add(1);
        if bit::check_hc(val, 1) {
            self.set_flag(Flag::H);
        } else {
            self.reset_flag(Flag::H);
        }
        if result == 0 {
            self.set_flag(Flag::Z);
        } else {
            self.reset_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);

        self.data_bus.assert(result);
        self.data_bus.write();
    }

    fn handle_dec_mem(&mut self) {
        let val = self.data_bus.read();
        let result = val.wrapping_sub(1);
        if bit::check_b(val, 1) {
            self.set_flag(Flag::H);
        } else {
            self.reset_flag(Flag::H);
        }
        if result == 0 {
            self.set_flag(Flag::Z);
        } else {
            self.reset_flag(Flag::Z);
        }
        self.set_flag(Flag::N);

        self.data_bus.assert(result);
        self.data_bus.write();
    }

    fn handle_inc_register(&mut self, r: instr::Register) {
        match r {
            instr::Register::BC => {
                self.bc().inc();
                return;
            }
            instr::Register::DE => {
                self.de().inc();
                return;
            }
            instr::Register::HL => {
                self.hl().inc();
                return;
            }
            instr::Register::SP => self.sp = self.sp.wrapping_add(1),
            instr::Register::A => {
                let val = self.a.val();
                let result = self.a.inc();
                if bit::check_hc(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.reset_flag(Flag::N);
            }
            instr::Register::B => {
                let val = self.b.val();
                let result = self.b.inc();
                if bit::check_hc(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.reset_flag(Flag::N);
            }
            instr::Register::D => {
                let val = self.d.val();
                let result = self.d.inc();
                if bit::check_hc(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.reset_flag(Flag::N);
            }
            instr::Register::H => {
                let val = self.h.val();
                let result = self.h.inc();
                if bit::check_hc(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.reset_flag(Flag::N);
            }
            instr::Register::C => {
                let val = self.c.val();
                let result = self.c.inc();
                if bit::check_hc(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.reset_flag(Flag::N);
            }
            instr::Register::E => {
                let val = self.e.val();
                let result = self.e.inc();
                if bit::check_hc(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.reset_flag(Flag::N);
            }
            instr::Register::L => {
                let val = self.l.val();
                let result = self.l.inc();
                if bit::check_hc(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.reset_flag(Flag::N);
            }
            _ => todo!("inc register {:?}", r),
        }
    }

    fn handle_dec_register(&mut self, r: instr::Register) {
        match r {
            instr::Register::BC => {
                self.bc().dec();
                return;
            }
            instr::Register::DE => {
                self.de().dec();
                return;
            }
            instr::Register::HL => {
                self.hl().dec();
                return;
            }
            instr::Register::SP => self.sp = self.sp.wrapping_sub(1),
            instr::Register::A => {
                let val = self.a.val();
                let result = self.a.dec();
                if bit::check_b(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.set_flag(Flag::N);
            }
            instr::Register::B => {
                let val = self.b.val();
                let result = self.b.dec();
                if bit::check_b(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.set_flag(Flag::N);
            }
            instr::Register::D => {
                let val = self.d.val();
                let result = self.d.dec();
                if bit::check_b(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.set_flag(Flag::N);
            }
            instr::Register::H => {
                let val = self.h.val();
                let result = self.h.dec();
                if bit::check_b(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.set_flag(Flag::N);
            }
            instr::Register::C => {
                let val = self.c.val();
                let result = self.c.dec();
                if bit::check_b(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.set_flag(Flag::N);
            }
            instr::Register::E => {
                let val = self.e.val();
                let result = self.e.dec();
                if bit::check_b(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.set_flag(Flag::N);
            }
            instr::Register::L => {
                let val = self.l.val();
                let result = self.l.dec();
                if bit::check_b(val, 1) {
                    self.set_flag(Flag::H);
                } else {
                    self.reset_flag(Flag::H);
                }
                if result == 0 {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
                self.set_flag(Flag::N);
            }
            _ => todo!("inc register {:?}", r),
        }
    }

    fn handle_assert(&mut self, r: instr::Register) {
        match r {
            instr::Register::BC => {
                let val = self.bc().val();
                self.addr_bus.assert(val);
            }
            instr::Register::DE => {
                let val = self.de().val();
                self.addr_bus.assert(val);
            }
            instr::Register::HLI => {
                let val = self.hl().val();
                self.hl().inc();
                self.addr_bus.assert(val);
            }
            instr::Register::HLD => {
                let val = self.hl().val();
                self.hl().dec();
                self.addr_bus.assert(val);
            }
            instr::Register::HL => {
                let val = self.hl().val();
                self.addr_bus.assert(val);
            }
            instr::Register::NN => {
                let val = self.ir.n16();
                self.addr_bus.assert(val);
            }
            instr::Register::SP => {
                self.addr_bus.assert(self.sp);
            }
            _ => todo!("assert register {:?}", r),
        }
    }

    fn handle_assert_inc(&mut self, r: instr::Register) {
        match r {
            instr::Register::NN => {
                let val = self.ir.n16().wrapping_add(1);
                self.addr_bus.assert(val);
            }
            instr::Register::SP => {
                let val = self.sp.wrapping_add(1);
                self.addr_bus.assert(val);
            }
            _ => todo!("assert inc register {:?}", r),
        }
    }

    fn handle_load(&mut self, l: Load) -> Result<(), Error> {
        match l {
            Load::Register(dst, src) => {
                let val = self.src_register(src)?;
                self.load_register(dst, val)?;
            }
            Load::Memory(src) => self.load_memory(src),
            Load::MemoryLo(src) => self.load_memory_lo(src),
            Load::MemoryHi(src) => self.load_memory_hi(src),
            //_ => todo!("load {:?}", l),
        }
        Ok(())
    }

    fn load_register(&mut self, dst: instr::Register, val: u16) -> Result<(), Error> {
        match dst {
            instr::Register::BC => self.bc().write(val),
            instr::Register::DE => self.de().write(val),
            instr::Register::HL => self.hl().write(val),
            instr::Register::SP => self.sp = val,
            instr::Register::PC => self.pc = val,
            instr::Register::B => self.b.write(val as u8),
            instr::Register::D => self.d.write(val as u8),
            instr::Register::H => self.h.write(val as u8),
            instr::Register::C => self.c.write(val as u8),
            instr::Register::E => self.e.write(val as u8),
            instr::Register::L => self.l.write(val as u8),
            instr::Register::A => self.a.write(val as u8),
            _ => todo!("register {:?}", dst),
        }
        Ok(())
    }

    fn load_memory(&mut self, src: instr::Register) {
        match src {
            instr::Register::A => {
                self.data_bus.assert(self.a.0);
                self.data_bus.write();
            }
            instr::Register::B => {
                self.data_bus.assert(self.b.0);
                self.data_bus.write();
            }
            instr::Register::C => {
                self.data_bus.assert(self.c.0);
                self.data_bus.write();
            }
            instr::Register::D => {
                self.data_bus.assert(self.d.0);
                self.data_bus.write();
            }
            instr::Register::E => {
                self.data_bus.assert(self.e.0);
                self.data_bus.write();
            }
            instr::Register::H => {
                self.data_bus.assert(self.h.0);
                self.data_bus.write();
            }
            instr::Register::L => {
                self.data_bus.assert(self.l.0);
                self.data_bus.write();
            }
            instr::Register::N => {
                self.data_bus.assert(self.ir.n8());
                self.data_bus.write();
            }
            instr::Register::NnLo => {
                self.data_bus.assert(self.ir.n16_lo());
                self.data_bus.write();
            }
            instr::Register::NnHi => {
                self.data_bus.assert(self.ir.n16_hi());
                self.data_bus.write();
            }
            _ => todo!("register {:?}", src),
        }
    }

    fn load_memory_lo(&mut self, src: instr::Register) {
        match src {
            instr::Register::SP => {
                let val = (self.sp & 0xFF) as u8;
                self.data_bus.assert(val);
                self.data_bus.write();
            }
            _ => todo!("register {:?}", src),
        }
    }

    fn load_memory_hi(&mut self, src: instr::Register) {
        match src {
            instr::Register::SP => {
                let val = (self.sp >> 8) as u8;
                self.data_bus.assert(val);
                self.data_bus.write();
            }
            _ => todo!("register {:?}", src),
        }
    }

    fn src_register(&mut self, r: instr::Register) -> Result<u16, Error> {
        match r {
            instr::Register::NN => Ok(self.ir.n16()),
            instr::Register::N => Ok(self.ir.n8() as u16),
            instr::Register::NE => Ok(self.ir.e() as u16),
            instr::Register::A => Ok(self.a.val() as u16),
            instr::Register::B => Ok(self.b.val() as u16),
            instr::Register::C => Ok(self.c.val() as u16),
            instr::Register::D => Ok(self.d.val() as u16),
            instr::Register::E => Ok(self.e.val() as u16),
            instr::Register::H => Ok(self.h.val() as u16),
            instr::Register::L => Ok(self.l.val() as u16),
            instr::Register::PC => Ok(self.pc),
            instr::Register::BC => Ok(self.bc().val()),
            instr::Register::DE => Ok(self.de().val()),
            instr::Register::SP => Ok(self.sp),
            instr::Register::HL => Ok(self.hl().val()),
            instr::Register::Memory => Ok(self.data_bus.read() as u16),
            _ => todo!("source register {:?}", r),
        }
    }

    fn handle_stop(&mut self) {
        self.stop = true;
    }

    pub fn tick(&mut self) -> Result<(), Error> {
        if self.stop {
            return Ok(());
        }
        if self.halt {
            return Ok(());
        }
        self.execute()?;
        if self.ir.done() {
            let opcode = self.fetch();
            self.ir = Instruction::decode(opcode);
            if self.ir.eager() {
                self.execute()?;
            }
        }
        Ok(())
    }

    fn jp(&mut self, r: R16) {
        todo!("jp")
        //let addr = self.src_r16(r);
        //self.pc = addr;
    }

    fn jp_cond(&mut self, c: Cond, r: R16) -> bool {
        if self.cc(c) {
            self.jp(r);
            return true;
        } else {
            self.fetch_word();
            return false;
        }
    }

    fn jr_word(&mut self, w: u16) {
        self.pc = self.pc.wrapping_add(w);
    }

    fn call(&mut self) {
        let pc = self.pc;
        self.push(pc);
        self.pc = self.fetch_word();
    }

    fn call_cond(&mut self, c: Cond) -> bool {
        if self.cc(c) {
            self.call();
            return true;
        }
        false
    }

    fn rst(&mut self, t: T3) {
        let pc = self.pc;
        let val = t.val();
        self.push(pc);
        self.pc = val as u16;
    }

    pub fn pop(&mut self, r: R16) {
        todo!("rework pop");
        //let sp = self.sp;
        //self.addr_bus.assert(addr);
        //self.sp = self.sp.wrapping_add(2);
        //let mut val = self.mem.read_word(sp);
        //if r == R16::AF {
        //    val &= 0xFFF0;
        //}
        //self.ld_r16(r, val)
    }

    pub fn push(&mut self, v: u16) {
        todo!("rework push")
        //self.sp -= 2;
        //let sp = self.sp;
        //self.mem.write_word(sp, v);
    }

    fn cc(&self, c: Cond) -> bool {
        match c {
            Cond::Z => self.flag(Flag::Z),
            Cond::NZ => !self.flag(Flag::Z),
            Cond::C => self.flag(Flag::C),
            Cond::NC => !self.flag(Flag::C),
        }
    }

    fn handle_check_cond(&mut self, c: Cond) {
        let cond = self.cc(c);
        if !cond {
            self.ir.complete();
        }
    }

    fn inc_r8(&mut self, r: R8) {
        todo!("rework or trash")
        //let val = self.src_r8(r);
        //let result = match r {
        //    R8::A | R8::B | R8::C | R8::D | R8::E | R8::H | R8::L => self.reg(r).inc(),
        //    R8::HL => {
        //        let addr = self.hl().val();
        //        self.mem.inc(addr)
        //    }
        //    _ => panic!("attempt to increment {}", r),
        //};
        //if result == 0 {
        //    self.reset_flag(Flag::Z);
        //}
        //self.reset_flag(Flag::N);
        //if bit::check_overflow(val, 1, 3) {
        //    self.set_flag(Flag::H);
        //}
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
                self.sp = self.sp.wrapping_add(1);
            }
            R16::PC => {
                self.pc = self.pc.wrapping_add(1);
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
        todo!("rework or trash")
        //let val = self.src_r8(r);
        //let result = match r {
        //    R8::A | R8::B | R8::C | R8::D | R8::E | R8::H | R8::L => self.reg(r).dec(),
        //    R8::HL => {
        //        let addr = self.hl().val();
        //        self.mem.dec(addr)
        //    }
        //    _ => panic!("attempt to increment {}", r),
        //};

        //if result == 0 {
        //    self.set_flag(Flag::Z);
        //}
        //if bit::check_borrow(val, 1, 4) {
        //    self.set_flag(Flag::H);
        //}
        //self.set_flag(Flag::N);
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

    fn handle_rlc(&mut self, r: instr::Register) {
        let val = self.src_register(r).unwrap() as u8;
        let b7 = bit::get(val, 7);
        self.set_flag_from_val(Flag::C, b7);
        let result = (val << 1) + b7;
        self.load_register(r, result as u16).unwrap();
        if r == instr::Register::A || result == 0 {
            self.reset_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
    }

    fn handle_rl(&mut self, r: instr::Register) {
        let val = self.src_register(r).unwrap() as u8;
        let cf = self.cf() as u8;
        let b7 = bit::get(val, 7);
        self.set_flag_from_val(Flag::C, b7);
        let result = (val << 1).wrapping_add(cf);
        self.load_register(r, result as u16).unwrap();

        if r == instr::Register::A || result == 0 {
            self.reset_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
    }

    fn handle_rrc(&mut self, r: instr::Register) {
        let val = self.src_register(r).unwrap() as u8;
        let b0 = bit::get(val, 0);
        self.set_flag_from_val(Flag::C, b0);
        let result = (val >> 1).wrapping_add(b0 << 7);
        self.load_register(r, result as u16).unwrap();

        if r == instr::Register::A || result == 0 {
            self.reset_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
    }

    fn handle_rr(&mut self, r: instr::Register) {
        let val = self.src_register(r).unwrap() as u8;
        let cf = self.cf() as u8;
        let b0 = bit::get(val, 0);
        self.set_flag_from_val(Flag::C, b0);
        let result = (val >> 1).wrapping_add(cf << 7);
        self.load_register(r, result as u16).unwrap();

        if r == instr::Register::A || result == 0 {
            self.reset_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
    }

    fn handle_daa(&mut self) {
        let a = self.a.val();
        let mut result: u8 = a;
        if self.flag(Flag::N) {
            if self.flag(Flag::H) {
                result = result.wrapping_sub(0x6);
            }
            if self.flag(Flag::C) {
                result = result.wrapping_sub(0x60);
            }
        } else {
            if self.flag(Flag::H) || (a & 0xF) > 0x9 {
                result = result.wrapping_add(0x6);
            }
            if self.flag(Flag::C) || a > 0x99 {
                result = result.wrapping_add(0x60);
                self.set_flag(Flag::C);
            }
        }
        self.a.write(result);
        self.set_flag_from_val(Flag::Z, (result == 0) as u8);
        self.reset_flag(Flag::H);
    }

    fn handle_scf(&mut self) {
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
        self.set_flag(Flag::C);
    }

    fn handle_cpl(&mut self) {
        let v = !self.a.val();
        self.a.write(v);
        self.set_flag(Flag::N);
        self.set_flag(Flag::H);
    }

    fn handle_ccf(&mut self) {
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
        self.invert_flag(Flag::C);
    }

    fn handle_add(&mut self, a: Add) {
        match a {
            Add::Register(a, b) => self.handle_add_register(a, b),
            _ => todo!("add {:?}", a),
        }
    }

    fn handle_adc(&mut self, a: ADC) {
        match a {
            ADC::Register(a, b) => self.handle_adc_register(a, b),
            _ => todo!("adc {:?}", a),
        }
    }

    fn handle_sub(&mut self, s: Sub) {
        match s {
            Sub::Register(a, b) => self.handle_sub_register(a, b),
            _ => todo!("sub {:?}", s),
        }
    }
    fn handle_sbc(&mut self, s: SBC) {
        match s {
            SBC::Register(a, b) => self.handle_sbc_register(a, b),
            _ => todo!("sub {:?}", s),
        }
    }

    fn handle_add_register(&mut self, r1: instr::Register, r2: instr::Register) {
        let a = self.src_register(r1).unwrap();
        let result: u16;
        if r2 == instr::Register::NE {
            let b = self.src_register(r2).unwrap() as i16;
            result = (a as i16).wrapping_add(b) as u16;
        } else {
            let b = self.src_register(r2).unwrap();
            result = a.wrapping_add(b);
            if self.check_half_carry(r1, r2) {
                self.set_flag(Flag::H);
            } else {
                self.reset_flag(Flag::H);
            }
            if self.check_carry(r1, r2) {
                self.set_flag(Flag::C);
            } else {
                self.reset_flag(Flag::C);
            }
            if r1 != instr::Register::HL {
                if self.check_zero(r1, r2, result) {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
            }
            self.reset_flag(Flag::N);
        }
        self.load_register(r1, result).unwrap();
    }

    fn handle_adc_register(&mut self, r1: instr::Register, r2: instr::Register) {
        let a = self.src_register(r1).unwrap();
        let c = match self.cf() {
            true => 1,
            false => 0,
        } as u16;
        let result: u16;
        if r2 == instr::Register::NE {
            let b = self.src_register(r2).unwrap() as i16;
            result = (a as i16).wrapping_add(b) as u16;
        } else {
            let b = self.src_register(r2).unwrap().wrapping_add(c);
            result = a.wrapping_add(b);
            if self.check_half_carry_adc(r1, r2, c) {
                self.set_flag(Flag::H);
            } else {
                self.reset_flag(Flag::H);
            }
            if self.check_carry_adc(r1, r2, c) {
                self.set_flag(Flag::C);
            } else {
                self.reset_flag(Flag::C);
            }
            if r1 != instr::Register::HL {
                if self.check_zero(r1, r2, result) {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
            }
            self.reset_flag(Flag::N);
        }
        self.load_register(r1, result).unwrap();
    }

    fn handle_sub_register(&mut self, r1: instr::Register, r2: instr::Register) {
        let a = self.src_register(r1).unwrap();
        let result: u16;
        println!("0x{:0>8b} : {}", self.f.0, self.f.0);
        if r2 == instr::Register::NE {
            let b = self.src_register(r2).unwrap() as i16;
            result = (a as i16).wrapping_add(b) as u16;
        } else {
            let b = self.src_register(r2).unwrap();
            result = a.wrapping_sub(b);
            if self.check_half_carry_sub(r1, r2) {
                self.set_flag(Flag::H);
            } else {
                self.reset_flag(Flag::H);
            }
            if self.check_carry_sub(r1, r2) {
                self.set_flag(Flag::C);
            } else {
                self.reset_flag(Flag::C);
            }
            if r1 != instr::Register::HL {
                if self.check_zero(r1, r2, result) {
                    self.set_flag(Flag::Z);
                } else {
                    self.reset_flag(Flag::Z);
                }
            }
            self.set_flag(Flag::N);
        }
        self.load_register(r1, result).unwrap();
    }

    fn handle_sbc_register(&mut self, r1: instr::Register, r2: instr::Register) {
        let a = self.src_register(r1).unwrap();
        let c = match self.cf() {
            true => 1,
            false => 0,
        } as u16;
        let result: u16;

        let b = self.src_register(r2).unwrap();
        result = a.wrapping_sub(b).wrapping_sub(c);
        if self.check_half_carry_sbc(r1, r2, c) {
            self.set_flag(Flag::H);
        } else {
            self.reset_flag(Flag::H);
        }
        if self.check_carry_sbc(r1, r2, c) {
            self.set_flag(Flag::C);
        } else {
            self.reset_flag(Flag::C);
        }
        if r1 != instr::Register::HL {
            if self.check_zero(r1, r2, result) {
                self.set_flag(Flag::Z);
            } else {
                self.reset_flag(Flag::Z);
            }
        }
        self.set_flag(Flag::N);

        self.load_register(r1, result).unwrap();
    }

    fn handle_and(&mut self, r: instr::Register) {
        let val = self.src_register(r).unwrap() as u8;
        let result = self.a.0 & val;
        if result == 0 {
            self.set_flag(Flag::Z);
        } else {
            self.reset_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.set_flag(Flag::H);
        self.reset_flag(Flag::C);

        self.a.write(result);
    }

    fn handle_xor(&mut self, r: instr::Register) {
        let val = self.src_register(r).unwrap() as u8;
        let result = self.a.0 ^ val;
        if result == 0 {
            self.set_flag(Flag::Z);
        } else {
            self.reset_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
        self.reset_flag(Flag::C);

        self.a.write(result);
    }

    fn handle_or(&mut self, r: instr::Register) {
        let val = self.src_register(r).unwrap() as u8;
        let result = self.a.0 | val;
        if result == 0 {
            self.set_flag(Flag::Z);
        } else {
            self.reset_flag(Flag::Z);
        }
        self.reset_flag(Flag::N);
        self.reset_flag(Flag::H);
        self.reset_flag(Flag::C);

        self.a.write(result);
    }

    fn check_zero(&mut self, a: instr::Register, b: instr::Register, result: u16) -> bool {
        if a.is_byte() && b.is_byte() {
            return (result as u8) == 0;
        } else {
            return result == 0;
        }
    }

    fn check_carry(&mut self, a: instr::Register, b: instr::Register) -> bool {
        if a.is_byte() && b.is_byte() {
            let a_val = self.src_register(a).unwrap() as u8;
            let b_val = self.src_register(b).unwrap() as u8;
            return bit::check_carry(a_val, b_val);
        } else {
            let a_val = self.src_register(a).unwrap();
            let b_val = self.src_register(b).unwrap();
            return bit::check_carry_word(a_val, b_val);
        }
    }

    fn check_carry_sub(&mut self, a: instr::Register, b: instr::Register) -> bool {
        if a.is_byte() && b.is_byte() {
            let a_val = self.src_register(a).unwrap();
            let b_val = self.src_register(b).unwrap();
            return a_val.wrapping_add((!b_val as u8) as u16) < 0xff;
        }
        panic!("sub not defined for 16 bit values");
    }

    fn check_carry_sbc(&mut self, a: instr::Register, b: instr::Register, c: u16) -> bool {
        if a.is_byte() && b.is_byte() {
            let a_val = self.src_register(a).unwrap();
            let b_val = self.src_register(b).unwrap();
            let res = a_val.wrapping_add((!b_val as u8) as u16);
            return res < 0xff || res.wrapping_sub(c) < 0xff;
        };
        panic!("sbc not defined for 16 bit values");
    }

    fn check_carry_adc(&mut self, a: instr::Register, b: instr::Register, c: u16) -> bool {
        if a.is_byte() && b.is_byte() {
            let a_val = self.src_register(a).unwrap() as u8;
            let b_val = self.src_register(b).unwrap() as u8;
            let result = a_val.wrapping_add(b_val);
            return bit::check_carry(a_val, b_val) || bit::check_carry(result, c as u8);
        }
        panic!("adc not defined for 16 bit values");
    }

    fn check_half_carry(&mut self, a: instr::Register, b: instr::Register) -> bool {
        if a.is_byte() && b.is_byte() {
            let a_val = self.src_register(a).unwrap() as u8;
            let b_val = self.src_register(b).unwrap() as u8;
            return bit::check_hc(a_val, b_val);
        } else {
            let a_val = self.src_register(a).unwrap();
            let b_val = self.src_register(b).unwrap();
            return bit::check_hc_word(a_val, b_val);
        }
    }

    fn check_half_carry_sub(&mut self, a: instr::Register, b: instr::Register) -> bool {
        if a.is_byte() && b.is_byte() {
            let a_val = self.src_register(a).unwrap() as u8;
            let b_val = self.src_register(b).unwrap() as u8;
            return (a_val & 0xf) + (!b_val & 0xf) < 0xf;
        } else {
            let a_val = self.src_register(a).unwrap();
            let b_val = self.src_register(b).unwrap();
            return (a_val & 0xff) + (!b_val & 0xff) < 0xff;
        }
    }

    fn check_half_carry_sbc(&mut self, a: instr::Register, b: instr::Register, c: u16) -> bool {
        if a.is_byte() && b.is_byte() {
            let a_val = self.src_register(a).unwrap() as u8;
            let b_val = self.src_register(b).unwrap() as u8;
            let res_a = (a_val & 0xf) + (!b_val & 0xf);
            return res_a < 0xf || res_a.wrapping_sub(c as u8) < 0xf;
        } else {
            let a_val = self.src_register(a).unwrap();
            let b_val = self.src_register(b).unwrap();
            return (a_val & 0xff)
                .wrapping_add(!(b_val & 0xff))
                .wrapping_add(!c & 0x1)
                < 0xff;
        }
    }

    fn check_half_carry_adc(&mut self, a: instr::Register, b: instr::Register, c: u16) -> bool {
        if a.is_byte() && b.is_byte() {
            let a_val = self.src_register(a).unwrap() as u8;
            let b_val = self.src_register(b).unwrap() as u8;
            let result = a_val.wrapping_add(b_val);
            return bit::check_hc(a_val, b_val) || bit::check_hc(result, c as u8);
        } else {
            let a_val = self.src_register(a).unwrap();
            let b_val = self.src_register(b).unwrap();
            let result = a_val.wrapping_add(b_val);
            return bit::check_hc_word(a_val, b_val) || bit::check_hc_word(result, c);
        }
    }

    fn check_half_carry_adc_sub(&mut self, a: instr::Register, b: instr::Register, c: u16) -> bool {
        if a.is_byte() && b.is_byte() {
            let a_val = self.src_register(a).unwrap() as u8;
            let b_val = self.src_register(b).unwrap() as u8;
            let result = a_val.wrapping_add(b_val);
            return bit::check_hc(a_val, !b_val) || bit::check_hc(result, c as u8);
        } else {
            let a_val = self.src_register(a).unwrap();
            let b_val = self.src_register(b).unwrap();
            let result = a_val.wrapping_add(b_val);
            return bit::check_hc_word(a_val, !b_val) || bit::check_hc_word(result, c);
        }
    }

    // ADC
    fn adc(&mut self, b: R8) {
        todo!("adc")
        //let a = self.a.val();
        //let cf = self.cf() as u8;
        //let val = self.src_r8(b);

        //let result = a.wrapping_add(val.wrapping_add(cf));
        //self.a.write(result);
        //if result == 0 {
        //    self.set_flag(Flag::Z);
        //}
        //self.reset_flag(Flag::N);
        //if bit::check_overflow(a, val.wrapping_add(cf), 3) {
        //    self.set_flag(Flag::H);
        //}
        //if bit::check_overflow(a, val.wrapping_add(cf), 7) {
        //    self.set_flag(Flag::C);
        //}
    }

    fn sub(&mut self, r: R8) {
        todo!("sub")
        //let a = self.a.val();
        //let val = self.src_r8(r);
        //let result = a.wrapping_sub(val);
        //self.a.write(result);

        //if result == 0 {
        //    self.set_flag(Flag::Z);
        //}
        //self.set_flag(Flag::N);
        //if bit::check_borrow(a, val, 4) {
        //    self.set_flag(Flag::H);
        //}
        //if bit::check_borrow(a, val, 8) {
        //    self.set_flag(Flag::C);
        //}
    }

    fn sbc(&mut self, r: R8) {
        todo!("sbc")
        //let a = self.a.val();
        //let cf = self.cf() as u8;
        //let val = self.src_r8(r);
        //let result = a.wrapping_sub(val.wrapping_add(cf));
        //self.a.write(result);

        //if result == 0 {
        //    self.set_flag(Flag::Z);
        //}
        //self.set_flag(Flag::N);
        //if bit::check_borrow(a, val.wrapping_add(cf), 4) {
        //    self.set_flag(Flag::H);
        //}
        //if bit::check_borrow(a, val.wrapping_add(cf), 8) {
        //    self.set_flag(Flag::C);
        //}
    }

    fn and(&mut self, r: R8) {
        todo!("and")
        //let a = self.a.val();
        //let val = self.src_r8(r);
        //let result = a & val;
        //self.a.write(result);

        //if result == 0 {
        //    self.set_flag(Flag::Z);
        //}
        //self.reset_flag(Flag::N);
        //self.set_flag(Flag::H);
        //self.reset_flag(Flag::C);
    }

    fn xor(&mut self, r: R8) {
        todo!("and")
        //let a = self.a.val();
        //let val = self.src_r8(r);
        //let result = a ^ val;
        //self.a.write(result);

        //if result == 0 {
        //    self.set_flag(Flag::Z);
        //}
        //self.reset_flag(Flag::N);
        //self.reset_flag(Flag::H);
        //self.reset_flag(Flag::C);
    }

    fn or(&mut self, r: R8) {
        todo!("or")
        //let a = self.a.val();
        //let val = self.src_r8(r);
        //let result = a | val;
        //self.a.write(result);

        //if result == 0 {
        //    self.set_flag(Flag::Z);
        //}
        //self.reset_flag(Flag::N);
        //self.reset_flag(Flag::H);
        //self.reset_flag(Flag::C);
    }

    fn cp(&mut self, r: R8) {
        todo!("cp")
        //let a = self.a.val();
        //let val = self.src_r8(r);
        //let result = a.wrapping_sub(val);

        //if result == 0 {
        //    self.set_flag(Flag::Z);
        //}
        //self.set_flag(Flag::N);
        //if bit::check_borrow(a, val, 4) {
        //    self.set_flag(Flag::H);
        //}
        //if bit::check_borrow(a, val, 8) {
        //    self.set_flag(Flag::C);
        //}
    }

    fn ldh_a(&mut self, m: instr::Mem) {
        todo!("rework")
        //let addr = match m {
        //    instr::Mem::C => (self.c.val() as u16) + 0xFF00,
        //    instr::Mem::N8 => (self.imm() as u16) + 0xFF00,
        //    _ => panic!("invalid ldh destination operation"),
        //};
        //let val = self.mem.read(addr);
        //self.a.write(val);
    }

    fn ldh_m(&mut self, m: instr::Mem) {
        todo!("rework")
        //let a = self.a.val();
        //let addr = match m {
        //    instr::Mem::C => (self.c.val() as u16) | 0xFF00,
        //    instr::Mem::N8 => (self.imm() as u16) | 0xFF00,
        //    _ => panic!("invalid ldh destination operation"),
        //};
        //self.mem.write(addr, a);
    }

    fn ret(&mut self) {
        todo!("rework")
        //let addr = self.sp;
        //let val = self.mem.read_word(addr);
        //self.pc = val;
        //self.sp = self.sp.wrapping_add(2);
    }

    fn ret_cond(&mut self, c: Cond) -> bool {
        if self.cc(c) {
            self.ret();
            return true;
        }
        false
    }

    //fn reti(&mut self) {
    //    self.ret();
    //    self.ime = true;
    //}

    fn sla(&mut self, r: R8) {
        todo!("sla")
        //let v = self.src_r8(r);
        //let v7 = bit::get(v, 7);
        //self.set_flag_from_val(Flag::C, v7);
        //let v = v << 1;
        //if v == 0 {
        //    self.set_flag(Flag::Z);
        //}
        //self.reset_flag(Flag::N);
        //self.reset_flag(Flag::H);
        //self.ld_r8(r, v);
    }

    fn sra(&mut self, r: R8) {
        todo!("sra")
        //let v = self.src_r8(r);
        //let v0 = bit::get(v, 0);
        //let v7 = bit::get(v, 7) << 7;

        //self.set_flag_from_val(Flag::C, v0);
        //let v = v >> 1 | v7;
        //if v == 0 {
        //    self.set_flag(Flag::Z);
        //}
        //self.reset_flag(Flag::N);
        //self.reset_flag(Flag::H);
        //self.ld_r8(r, v);
    }

    fn swap(&mut self, r: R8) {
        todo!("swap")
        //let v = self.src_r8(r);
        //let l = (v & 0b1111) << 4;
        //let h = (v & 0b1111_0000) >> 4;
        //let v = l | h;
        //self.ld_r8(r, v);
        //if v == 0 {
        //    self.set_flag(Flag::Z);
        //}
        //self.reset_flag(Flag::N);
        //self.reset_flag(Flag::H);
        //self.reset_flag(Flag::C);
    }

    fn srl(&mut self, r: R8) {
        todo!("srl")
        //let v = self.src_r8(r);
        //let v0 = bit::get(v, 0);
        //self.set_flag_from_val(Flag::C, v0);
        //let v = v >> 1;

        //if v == 0 {
        //    self.set_flag(Flag::Z);
        //}
        //self.reset_flag(Flag::N);
        //self.reset_flag(Flag::H);
        //self.ld_r8(r, v);
    }

    fn bit(&mut self, b: B3, r: R8) {
        todo!("bit")
        //let val = self.src_r8(r);
        //let i = b.val();

        //if bit::is_set(val, i) {
        //    self.set_flag(Flag::Z);
        //}
        //self.reset_flag(Flag::N);
        //self.set_flag(Flag::H);
    }

    fn res(&mut self, b: B3, r: R8) {
        todo!("bit")
        //let mut val = self.src_r8(r);
        //let i = b.val();
        //bit::reset(&mut val, i);
        //self.ld_r8(r, val);
    }

    fn set(&mut self, b: B3, r: R8) {
        todo!("set")
        //let mut val = self.src_r8(r);
        //let i = b.val();
        //bit::set(&mut val, i);
        //self.ld_r8(r, val);
    }

    fn query_interrupt(&mut self) -> Option<Interrupt> {
        todo!("query_interrupt");
        //let mut mem = self.mem();
        //let iflags = mem.interrupt_flags();
        //if iflags.vblank() {
        //    return Some(Interrupt::VBlank);
        //}
        //if iflags.lcd() {
        //    return Some(Interrupt::STAT);
        //}
        //if iflags.timer() {
        //    return Some(Interrupt::Timer);
        //}
        //if iflags.serial() {
        //    return Some(Interrupt::Serial);
        //}
        //if iflags.joypad() {
        //    return Some(Interrupt::Joypad);
        //}
        //None
    }

    fn handle_interrupt(&mut self, i: Interrupt) -> u8 {
        todo!("handle_interrupt");
        //// 8 t cycles
        //self.mem().interrupt_flags().reset(i);
        //// 8 t cycles
        //self.ime = false;
        //self.push(self.pc);
        //// 4 t cycles
        //self.pc = i.addr() as u16;
        //20
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use crate::io::DIV;

    use super::*;

    fn setup() -> (CPU, Memory) {
        let mem = Memory::new();
        let cpu = CPU::new(&mem);
        (cpu, mem)
    }

    fn load_test(file_name: String) -> Vec<Test> {
        let path = String::from("./test_files/v1/") + file_name.as_str();
        let raw = fs::read_to_string(path).unwrap();
        serde_json::from_str(raw.as_str()).unwrap()
    }

    fn cmp_mem_state(mem: &mut Memory, test_state: &Vec<[u16; 2]>) {
        for state in test_state.iter() {
            assert_eq!(mem.read(state[0]), state[1] as u8);
        }
    }

    fn cmp_cpu_state(cpu: &mut CPU, s: &State) {
        assert_eq!(s.a, cpu.a.0);
        println!("0b{:0>8b} (expected)", s.f);
        println!("0b{:0>8b} (actual)", cpu.f.0);
        assert_eq!(s.f, cpu.f.0);
        assert_eq!(s.b, cpu.b.0);
        assert_eq!(s.c, cpu.c.0);
        assert_eq!(s.d, cpu.d.0);
        assert_eq!(s.e, cpu.e.0);
        assert_eq!(s.h, cpu.h.0);
        assert_eq!(s.l, cpu.l.0);
        assert_eq!(s.pc, cpu.pc);
        assert_eq!(s.sp, cpu.sp);
        //assert_eq!(s.ime, cpu.ime as u8);
        //if let Some(ie) = s.ie {
        //    assert_eq!(ie, cpu.ie as u8);
        //} else {
        //    assert!(!cpu.ie);
        //}
    }

    fn run_json_test(test: String) {
        let tests = load_test(test);
        for test in tests {
            println!("sm83 {}", test.name);
            println!("test: {:?}", test);
            let (mut cpu, mut mem) = setup();
            cpu.load_state(&test.initial);
            mem.load_state(&test.initial.ram);

            for (i, mc) in test.cycles.iter().enumerate() {
                println!("cycle {}", i);
                println!("{:?}", mc);
                cpu.tick().unwrap();
                if let Some(addr) = mc.addr {
                    println!("\t----");
                    println!("\texpected addr: 0x{:x}", addr);
                    println!("\tactual addr: 0x{:x}", cpu.addr_bus.current());
                    assert_eq!(addr, cpu.addr_bus.current());
                }
                if let Some(d) = mc.data {
                    println!("\t----");
                    println!("\texpected data: {:?}", d);
                    println!("\tactual data: {:?}", cpu.data_bus.read());
                    assert_eq!(d, cpu.data_bus.read());
                }
            }
            cmp_mem_state(&mut mem, &test.r#final.ram);
            cmp_cpu_state(&mut cpu, &test.r#final);
        }
    }

    #[test]
    fn fetch() {
        let (mut cpu, mut mem) = setup();
        let val = 0xCC;
        mem.write(cpu.pc, val);
        let pc = cpu.pc;
        let fetched = cpu.fetch();
        assert_eq!(val, fetched);
        assert_eq!(cpu.pc, pc + 1);
    }

    #[test]
    fn test_nop() {
        // NOP
        run_json_test(String::from("00.json"));
    }

    #[test]
    fn test_stop() {
        run_json_test(String::from("10.json"));
    }

    #[test]
    fn test_halt() {
        run_json_test(String::from("76.json"));
    }

    #[test]
    fn test_ld_r16_nn() {
        // LD BC, n16
        run_json_test(String::from("01.json"));
        // LD DE, n16
        run_json_test(String::from("11.json"));
        // LD HL, n16
        run_json_test(String::from("21.json"));
        // LD SP, n16
        run_json_test(String::from("31.json"));
    }

    #[test]
    fn test_ld_r16mem_a() {
        // LD (BC), A
        run_json_test(String::from("02.json"));
        // LD (DE), A
        run_json_test(String::from("12.json"));
        // LD (HL+), A
        run_json_test(String::from("22.json"));
        // LD (HL-), A
        run_json_test(String::from("32.json"));
    }

    #[test]
    fn test_inc_r16() {
        // INC BC
        run_json_test(String::from("03.json"));
        // INC DE
        run_json_test(String::from("13.json"));
        // INC HL
        run_json_test(String::from("23.json"));
        // INC SP
        run_json_test(String::from("33.json"));
    }

    #[test]
    fn test_inc_r() {
        // INC B
        run_json_test(String::from("04.json"));
        // INC D
        run_json_test(String::from("14.json"));
        // INC H
        run_json_test(String::from("24.json"));
        // INC (HL)
        run_json_test(String::from("34.json"));
        // INC C
        run_json_test(String::from("0c.json"));
        // INC E
        run_json_test(String::from("1c.json"));
        // INC L
        run_json_test(String::from("2c.json"));
        // INC A
        run_json_test(String::from("3c.json"));
    }

    #[test]
    fn test_dec_r() {
        // DEC B
        run_json_test(String::from("05.json"));
        // DEC D
        run_json_test(String::from("15.json"));
        // DEC H
        run_json_test(String::from("25.json"));
        // DEC (HL)
        run_json_test(String::from("35.json"));
        // DEC C
        run_json_test(String::from("0d.json"));
        // DEC E
        run_json_test(String::from("1d.json"));
        // DEC L
        run_json_test(String::from("2d.json"));
        // DEC A
        run_json_test(String::from("3d.json"));
    }

    #[test]
    fn test_ld_r8_n() {
        // LD B, n
        run_json_test(String::from("06.json"));
        // LD D, n
        run_json_test(String::from("16.json"));
        // LD H, n
        run_json_test(String::from("26.json"));
        // LD (HL), n
        run_json_test(String::from("36.json"));

        // LD C, n
        run_json_test(String::from("0e.json"));
        // LD E, n
        run_json_test(String::from("1e.json"));
        // LD L, n
        run_json_test(String::from("2e.json"));
        // LD A, n
        run_json_test(String::from("3e.json"));
    }

    #[test]
    fn test_rlca() {
        run_json_test(String::from("07.json"));
    }

    #[test]
    fn test_rla() {
        run_json_test(String::from("17.json"));
    }

    #[test]
    fn test_daa() {
        run_json_test(String::from("27.json"));
    }

    #[test]
    fn test_scf() {
        run_json_test(String::from("37.json"));
    }

    #[test]
    fn test_rrca() {
        run_json_test(String::from("0f.json"));
    }

    #[test]
    fn test_rra() {
        run_json_test(String::from("1f.json"));
    }

    #[test]
    fn test_cpl() {
        run_json_test(String::from("2f.json"));
    }

    #[test]
    fn test_ccf() {
        run_json_test(String::from("3f.json"));
    }

    #[test]
    fn test_ld_nnm_sp() {
        run_json_test(String::from("08.json"));
    }

    #[test]
    fn test_jr() {
        run_json_test(String::from("18.json"));
    }

    #[test]
    fn test_jr_cc() {
        // JR NZ
        run_json_test(String::from("20.json"));
        // JR NC
        run_json_test(String::from("30.json"));
        // JR Z
        run_json_test(String::from("28.json"));
        // JR C
        run_json_test(String::from("38.json"));
    }

    #[test]
    fn test_add_hl_rr() {
        // add HL, BC
        run_json_test(String::from("09.json"));
        // add HL, DE
        run_json_test(String::from("19.json"));
        // add HL, HL
        run_json_test(String::from("29.json"));
        // add HL, SP
        run_json_test(String::from("39.json"));
    }

    #[test]
    fn test_ld_r_r() {
        // ld B/C, R
        for i in 0x0..=0xF {
            let file = format!("4{:x}.json", i);
            run_json_test(file);
        }

        // ld D/E, R
        for i in 0x0..=0xF {
            let file = format!("5{:x}.json", i);
            run_json_test(file);
        }

        // ld H/L, R
        for i in 0x0..=0xF {
            let file = format!("6{:x}.json", i);
            run_json_test(file);
        }

        // ld (HL), R
        for i in 0x0..=0x7 {
            if i == 0x6 {
                continue;
            }
            let file = format!("7{:x}.json", i);
            run_json_test(file);
        }

        // ld A, R
        for i in 0x8..=0xF {
            let file = format!("7{:x}.json", i);
            run_json_test(file);
        }
    }

    #[test]
    fn test_add_a_r() {
        // add A, R
        for i in 0x0..=0x7 {
            let file = format!("8{:x}.json", i);
            run_json_test(file);
        }
    }

    #[test]
    fn test_adc_a_r() {
        // adc A, R
        for i in 0x8..=0xF {
            let file = format!("8{:x}.json", i);
            run_json_test(file);
        }
    }

    #[test]
    fn test_sub_a_r() {
        // sub A, R
        for i in 0x0..=0x7 {
            let file = format!("9{:x}.json", i);
            run_json_test(file);
        }
    }

    #[test]
    fn test_sbc_a_r() {
        // sbc A, R
        for i in 0x8..=0xF {
            let file = format!("9{:x}.json", i);
            run_json_test(file);
        }
    }

    #[test]
    fn test_and_a_r() {
        // AND A, R
        for i in 0x0..=0x7 {
            let file = format!("a{:x}.json", i);
            run_json_test(file);
        }
    }

    #[test]
    fn test_xor_a_r() {
        // XOR A, R
        for i in 0x8..=0xF {
            let file = format!("a{:x}.json", i);
            run_json_test(file);
        }
    }

    #[test]
    fn test_or_a_r() {
        // OR A, R
        for i in 0x0..=0x7 {
            let file = format!("b{:x}.json", i);
            run_json_test(file);
        }
    }

    //#[test]
    //fn jump_relative() {
    //    let mem = Arc::new(Mutex::new(Memory::new()));
    //    let mut cpu = CPU::new(&mem);
    //    let byte = 0xFF;
    //    let expected = cpu.pc + (byte as u16);
    //    cpu.jr(byte);
    //    assert_eq!(cpu.pc, expected);
    //}

    //#[test]
    //fn jump_relative_word() {
    //    let mem = Memory::arc();
    //    let mut cpu = CPU::new(&mem);
    //    let word = 0x00FF;
    //    let expected = cpu.pc + word;
    //    cpu.jr_word(word);
    //    assert_eq!(cpu.pc, expected);
    //}

    //#[test]
    //fn af() {
    //    let mem = Arc::new(Mutex::new(Memory::new()));
    //    let mut cpu = CPU::new(&mem);
    //    let byte = 0b0110_0110;
    //    let expected = ((byte as u16) << 8) | 0b1000_0000;
    //    cpu.set_flag(Flag::Z);
    //    cpu.a.write(byte);
    //    assert_eq!(cpu.af().val(), expected);
    //}

    #[test]
    fn flag_set() {
        let mem = Memory::new();
        let mut cpu = CPU::new(&mem);
        cpu.set_flag(Flag::N);
        assert!(cpu.nf());
    }

    #[test]
    fn flag_reset() {
        let mem = Memory::new();
        let mut cpu = CPU::new(&mem);
        cpu.set_flag(Flag::N);
        assert!(cpu.nf());
        cpu.reset_flag(Flag::N);
        assert!(!cpu.nf());
    }

    #[test]
    fn flag() {
        let mem = Memory::new();
        let mut cpu = CPU::new(&mem);
        assert!(!cpu.flag(Flag::N));
        cpu.set_flag(Flag::N);
        assert!(cpu.flag(Flag::N));
    }

    #[test]
    fn flag_set_val() {
        let mem = Memory::new();
        let mut cpu = CPU::new(&mem);
        let f = Flag::C;
        cpu.set_flag_from_val(f, 1);
        assert!(cpu.cf());
        cpu.set_flag_from_val(f, 0);
        assert!(!cpu.cf());
    }

    #[test]
    fn cc() {
        let mem = Memory::new();
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

        expected_word = expected_word.wrapping_add(1);
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

    #[test]
    fn deserialize_cycle_state() {
        // valid
        let cs_str = r#"[24525,174,"r-m"]"#;
        let cs: CycleState = serde_json::from_str(cs_str).unwrap();
        assert_eq!(Some(24525), cs.addr);
        assert_eq!(Some(174), cs.data);
        assert!(cs.r);
        assert!(!cs.w);
        assert!(cs.m);

        // valid, includes nulls
        let cs_str = r#"[null,null,"-w-"]"#;
        let cs: CycleState = serde_json::from_str(cs_str).unwrap();
        assert_eq!(None, cs.addr);
        assert_eq!(None, cs.data);
        assert!(!cs.r);
        assert!(cs.w);
        assert!(!cs.m);

        // invalid 0
        let cs_str = r#"[null,null,"-w-foo"]"#;
        let should_fail: Result<CycleState, serde_json::Error> = serde_json::from_str(cs_str);
        if let Ok(_) = should_fail {
            panic!("expected string to fail deserialization");
        }

        // invalid 1
        let cs_str = r#"[null,null,"-w-o"#;
        let should_fail: Result<CycleState, serde_json::Error> = serde_json::from_str(cs_str);
        if let Ok(_) = should_fail {
            panic!("expected string to fail deserialization");
        }

        // invalid 2
        let cs_str = r#"[null,"string","rwm"]"#;
        let should_fail: Result<CycleState, serde_json::Error> = serde_json::from_str(cs_str);
        if let Ok(_) = should_fail {
            panic!("expected string to fail deserialization");
        }
    }
}
