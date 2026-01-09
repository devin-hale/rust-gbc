use std::{error::Error, fmt::Display};

use thiserror::Error;

use crate::{
    bit,
    cpu::{CPU, Flag},
};

pub enum Block {
    Block0,
    Block1,
    Block2,
    Block3,
    Prefix,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operand {
    B,
    C,
    D,
    E,
    H,
    L,
    A,
    BC,
    DE,
    HL,
    SP,
    AF,
    HLI,
    HLD,
    Imm8,
    Imm16,
    Cond(Cond),
    B3(BitIndex),
    Tgt3(Tgt3),
    Sum((Box<Operand>, Box<Operand>)),
    Mem(Box<Operand>),
}

impl Operand {
    fn to_mem(&self) -> Operand {
        match self {
            Operand::Mem(_) => self.clone(),
            _ => Operand::Mem(Box::new(self.clone())),
        }
    }
}

impl From<R8> for Operand {
    fn from(value: R8) -> Self {
        match value {
            R8::B => Operand::B,
            R8::C => Operand::C,
            R8::D => Operand::D,
            R8::E => Operand::E,
            R8::H => Operand::H,
            R8::L => Operand::L,
            R8::HlMem => Operand::HL.to_mem(),
            R8::A => Operand::A,
        }
    }
}

impl Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let src = match self {
            Operand::B => format!("b"),
            Operand::C => format!("c"),
            Operand::D => format!("d"),
            Operand::E => format!("e"),
            Operand::H => format!("h"),
            Operand::L => format!("l"),
            Operand::A => format!("a"),
            Operand::BC => format!("bc"),
            Operand::DE => format!("de"),
            Operand::HL => format!("hl"),
            Operand::SP => format!("sp"),
            Operand::AF => format!("af"),
            Operand::HLI => format!("hl+"),
            Operand::HLD => format!("hl-"),
            Operand::Cond(c) => format!("{}", c),
            Operand::B3(bi) => format!("{}", bi),
            Operand::Tgt3(t3) => format!("{}", t3),
            Operand::Imm8 => String::from("imm8"),
            Operand::Imm16 => String::from("imm16"),
            Operand::Sum((a, b)) => format!("{} + {}", *a, *b),
            Operand::Mem(b) => format!("[{}]", *b),
        };
        write!(f, "{}", src)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum R8 {
    B,
    C,
    D,
    E,
    H,
    L,
    HlMem,
    A,
}

impl R8 {
    fn from_u8(b: u8) -> Result<R8, InstructionError> {
        match b {
            0 => Ok(R8::B),
            1 => Ok(R8::C),
            2 => Ok(R8::D),
            3 => Ok(R8::E),
            4 => Ok(R8::H),
            5 => Ok(R8::L),
            6 => Ok(R8::HlMem),
            7 => Ok(R8::A),
            _ => Err(InstructionError::InvalidR8(b)),
        }
    }
}

impl Display for R8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r8: &'static str = match self {
            R8::B => "b",
            R8::C => "c",
            R8::D => "d",
            R8::E => "e",
            R8::H => "h",
            R8::L => "l",
            R8::HlMem => "[hl]",
            R8::A => "a",
        };
        write!(f, "{}", r8)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum R16 {
    BC,
    DE,
    HL,
    SP,
}

impl R16 {
    fn from_u8(b: u8) -> Result<R16, InstructionError> {
        match b {
            0 => Ok(R16::BC),
            1 => Ok(R16::DE),
            2 => Ok(R16::HL),
            3 => Ok(R16::SP),
            _ => Err(InstructionError::InvalidR16(b)),
        }
    }

    fn to_operand(&self) -> Operand {
        match self {
            R16::BC => Operand::BC,
            R16::DE => Operand::DE,
            R16::HL => Operand::HL,
            R16::SP => Operand::SP,
        }
    }
}

impl Display for R16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r16: &'static str = match self {
            R16::BC => "bc",
            R16::DE => "de",
            R16::HL => "hl",
            R16::SP => "sp",
        };
        write!(f, "{}", r16)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum R16Stk {
    BC,
    DE,
    HL,
    AF,
}

impl R16Stk {
    fn from_u8(b: u8) -> Result<R16Stk, InstructionError> {
        match b {
            0 => Ok(R16Stk::BC),
            1 => Ok(R16Stk::DE),
            2 => Ok(R16Stk::HL),
            3 => Ok(R16Stk::AF),
            _ => Err(InstructionError::InvalidR16Stk(b)),
        }
    }

    fn to_operand(&self) -> Operand {
        match self {
            R16Stk::BC => Operand::BC,
            R16Stk::DE => Operand::DE,
            R16Stk::HL => Operand::HL,
            R16Stk::AF => Operand::AF,
        }
    }
}

impl Display for R16Stk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r16: &'static str = match self {
            R16Stk::BC => "bc",
            R16Stk::DE => "de",
            R16Stk::HL => "hl",
            R16Stk::AF => "af",
        };
        write!(f, "{}", r16)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum R16Mem {
    BC,
    DE,
    HLI,
    HLD,
}

impl R16Mem {
    fn from_u8(b: u8) -> Result<R16Mem, InstructionError> {
        match b {
            0 => Ok(R16Mem::BC),
            1 => Ok(R16Mem::DE),
            2 => Ok(R16Mem::HLI),
            3 => Ok(R16Mem::HLD),
            _ => Err(InstructionError::InvalidR16Mem(b)),
        }
    }

    fn to_operand(&self) -> Operand {
        match self {
            R16Mem::BC => Operand::BC,
            R16Mem::DE => Operand::BC,
            R16Mem::HLI => Operand::HLI,
            R16Mem::HLD => Operand::HLD,
        }
    }
}

impl Display for R16Mem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r16: &'static str = match self {
            R16Mem::BC => "bc",
            R16Mem::DE => "de",
            R16Mem::HLI => "hl+",
            R16Mem::HLD => "hl-",
        };
        write!(f, "{}", r16)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cond {
    NZ,
    Z,
    NC,
    C,
}

impl Cond {
    fn from_u8(b: u8) -> Result<Cond, InstructionError> {
        match b {
            0 => Ok(Cond::NZ),
            1 => Ok(Cond::Z),
            2 => Ok(Cond::NC),
            3 => Ok(Cond::C),
            _ => Err(InstructionError::InvalidCond(b)),
        }
    }

    fn to_operand(&self) -> Operand {
        match self {
            Cond::NZ => Operand::Cond(Cond::NZ),
            Cond::Z => Operand::Cond(Cond::Z),
            Cond::NC => Operand::Cond(Cond::NC),
            Cond::C => Operand::Cond(Cond::C),
        }
    }
}
impl Display for Cond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r16: &'static str = match self {
            Cond::NZ => "nz",
            Cond::Z => "z",
            Cond::NC => "nc",
            Cond::C => "c",
        };
        write!(f, "{}", r16)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BitIndex {
    index: u8,
}

impl BitIndex {
    pub fn new(index: u8) -> BitIndex {
        BitIndex {
            index: index & 0b111,
        }
    }

    pub fn index(&self) -> u8 {
        self.index
    }

    pub fn to_operand(&self) -> Operand {
        Operand::B3(self.clone())
    }
}

impl Display for BitIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.index)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tgt3 {
    addr: u8,
}

impl Tgt3 {
    pub fn new(addr: u8) -> Tgt3 {
        Tgt3 { addr: addr / 8 }
    }

    pub fn addr(&self) -> u8 {
        self.addr
    }

    pub fn to_operand(&self) -> Operand {
        Operand::Tgt3(self.clone())
    }
}

impl From<Option<Operand>> for Tgt3 {
    fn from(value: Option<Operand>) -> Self {
        match value {
            Some(o) => match o {
                Operand::Tgt3(t) => t,
                _ => panic!("operand canont be converted to Tgt3: {}", o),
            },
            _ => panic!("empty operand"),
        }
    }
}

impl From<Tgt3> for u8 {
    fn from(value: Tgt3) -> Self {
        value.addr()
    }
}

impl Display for Tgt3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.addr)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operation {
    NOP,
    LD,
    INC,
    DEC,
    ADD,
    RLCA,
    RRCA,
    RLA,
    RRA,
    DAA,
    CPL,
    SCF,
    CCF,
    JR,
    STOP,
    HALT,
    ADC,
    SUB,
    SBC,
    AND,
    XOR,
    OR,
    CP,
    RET,
    RETI,
    JP,
    CALL,
    RST,
    POP,
    PUSH,
    LDH,
    DI,
    EI,
    RR,
    RLC,
    RRC,
    RL,
    SLA,
    SRA,
    SWAP,
    SRL,
    BIT,
    RES,
    SET,
    PREFIX,
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            Operation::NOP => "nop",
            Operation::LD => "ld",
            Operation::INC => "inc",
            Operation::DEC => "dec",
            Operation::ADD => "add",
            Operation::RLCA => "rlca",
            Operation::RRCA => "rrca",
            Operation::RLA => "rla",
            Operation::RRA => "rra",
            Operation::DAA => "daa",
            Operation::CPL => "cpl",
            Operation::SCF => "scf",
            Operation::CCF => "ccf",
            Operation::JR => "jr",
            Operation::STOP => "stop",
            Operation::HALT => "halt",
            Operation::ADC => "adc",
            Operation::SUB => "sub",
            Operation::SBC => "sbc",
            Operation::AND => "and",
            Operation::XOR => "xor",
            Operation::OR => "or",
            Operation::CP => "cp",
            Operation::RET => "ret",
            Operation::RETI => "reti",
            Operation::JP => "jp",
            Operation::CALL => "call",
            Operation::RST => "rst",
            Operation::POP => "pop",
            Operation::PUSH => "push",
            Operation::LDH => "ldh",
            Operation::DI => "di",
            Operation::EI => "ei",
            Operation::RR => "rr",
            Operation::RLC => "rlc",
            Operation::RRC => "rrc",
            Operation::RL => "rl",
            Operation::SLA => "sla",
            Operation::SRA => "sra",
            Operation::SWAP => "swap",
            Operation::SRL => "srl",
            Operation::BIT => "bit",
            Operation::RES => "res",
            Operation::SET => "set",
            Operation::PREFIX => "cb prefix",
        };
        write!(f, "{}", op)
    }
}

type Executor = fn(&mut Instruction, &mut CPU) -> Result<(), Box<dyn Error>>;

pub struct Instruction {
    op: Operation,
    dest: Option<Operand>,
    src: Option<Operand>,

    length: u8,
    cycles: u8,
    branch_cycles: u8,

    ex: Executor,
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut instr = format!("{}", self.op);
        if let Some(o) = self.dest() {
            instr = format!("{} {}", instr, o);
        }
        if let Some(o) = self.src() {
            instr = format!("{}, {}", instr, o);
        }
        write!(f, "{}", instr)
    }
}

impl Instruction {
    pub fn nop() -> Instruction {
        Instruction {
            op: Operation::NOP,
            dest: None,
            src: None,
            length: 1,
            cycles: 4,
            branch_cycles: 0,
            ex: |_, _| Ok(()),
        }
    }
    pub fn stop() -> Instruction {
        let mut i = Instruction::nop();
        i.op = Operation::STOP;
        i.ex = |_, cpu| {
            cpu.fetch()?;
            cpu.stop();
            Ok(())
        };
        i
    }
    pub fn halt() -> Instruction {
        let mut i = Instruction::nop();
        i.op = Operation::HALT;
        i.ex = |_, cpu| {
            cpu.halt();
            Ok(())
        };
        i
    }
    pub fn new() -> Instruction {
        Instruction::nop()
    }

    pub fn op(&self) -> Operation {
        self.op
    }

    pub fn dest(&self) -> Option<Operand> {
        self.dest.clone()
    }

    pub fn src(&self) -> Option<Operand> {
        self.src.clone()
    }
}

#[derive(Debug, Error)]
pub enum InstructionError {
    #[error("invalid R16 value '{0}'")]
    InvalidR16(u8),

    #[error("invalid R16Mem value '{0}'")]
    InvalidR16Mem(u8),

    #[error("invalid R16Stk value '{0}'")]
    InvalidR16Stk(u8),

    #[error("invalid R8 value '{0}")]
    InvalidR8(u8),

    #[error("invalid Cond value '{0}")]
    InvalidCond(u8),

    #[error("unimplemented instruction '{0:b}")]
    Unimplemented(u8),

    #[error("invalid opcode '{0:x}")]
    InvalidOpCode(u8),

    #[error("unknown instruction error")]
    Unknown,
}

pub fn decode(opcode: u8) -> Result<Instruction, InstructionError> {
    match (opcode >> 6) & 0x3 {
        0x00 => Ok(decode_block_0(opcode)?),
        0x01 => Ok(decode_block_1(opcode)?),
        0x10 => Ok(decode_block_2(opcode)?),
        0x11 => {
            let i = decode_block_3(opcode)?;
            if i.op == Operation::PREFIX {
                return Ok(decode_prefix(opcode)?);
            } else {
                return Ok(i);
            }
        }
        _ => Err(InstructionError::Unknown),
    }
}

#[allow(unused_variables)]
fn decode_block_0(opcode: u8) -> Result<Instruction, InstructionError> {
    if opcode == 0x00 {
        return Ok(Instruction::nop());
    }
    if opcode == 0xF0 {
        return Ok(Instruction::stop());
    }

    let mut i = Instruction::new();
    let last_four = opcode & 0b1111;
    match last_four {
        // ld r16, imm16
        0b0001 => {
            i.op = Operation::LD;
            let dest = R16::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.dest = Some(dest.to_operand());
            i.src = Some(Operand::Imm16);
            i.length = 3;
            i.cycles = 12;
            i.ex = |i, cpu| {
                let imm16 = cpu.fetch_word()?;
                cpu.register(i.dest())?.write(imm16);
                Ok(())
            };
            return Ok(i);
        }
        // ld [r16mem], a
        0b0010 => {
            i.op = Operation::LD;
            let dest = R16Mem::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.dest = Some(dest.to_operand().to_mem());
            i.src = Some(R8::A.into());
            i.length = 1;
            i.cycles = 8;
            i.ex = |i, cpu| {
                let addr = cpu.register(i.dest())?.val();
                let a = cpu.read_byte(i.src())?;
                let mu = cpu.mem();
                let mut mem = mu.lock().unwrap();
                mem.write(addr, a)?;
                Ok(())
            };
            return Ok(i);
        }
        // ld a, [r16mem]
        0b1010 => {
            i.op = Operation::LD;
            let dest = R16Mem::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.dest = Some(R8::A.into());
            i.src = Some(dest.to_operand().to_mem());
            i.length = 1;
            i.cycles = 8;
            i.ex = |i, cpu| {
                let addr = cpu.register(i.src())?.val();
                let mu = cpu.mem();
                let mem = mu.lock().unwrap();
                let val = mem.read(addr)?.clone();
                cpu.write_byte(i.dest(), val)?;
                Ok(())
            };
            return Ok(i);
        }
        // ld [imm16], sp
        0b1000 => {
            let dest = (opcode & 0b0011_0000) >> 4;
            if dest == 0b00 {
                let dest = R16Mem::from_u8(dest)?;
                i.op = Operation::LD;
                i.dest = Some(Operand::Imm16.to_mem());
                i.src = Some(R16::SP.to_operand());
                i.length = 3;
                i.cycles = 20;
                i.ex = |i, cpu| {
                    let val = cpu.register(i.src())?.val();
                    let imm16 = cpu.fetch_word()?;
                    let mu = cpu.mem();
                    let mut mem = mu.lock().unwrap();
                    mem.write_word(imm16, val)?;
                    Ok(())
                };
                return Ok(i);
            }
        }

        // inc r16
        0b0011 => {
            i.op = Operation::INC;
            let r = R16::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.dest = Some(r.to_operand());
            i.length = 1;
            i.cycles = 8;

            i.ex = |i, cpu| {
                cpu.register(i.dest())?.inc();
                Ok(())
            };

            return Ok(i);
        }
        // dec r16
        0b1011 => {
            i.op = Operation::DEC;
            let r = R16::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.dest = Some(r.to_operand());
            i.length = 1;
            i.cycles = 8;
            i.ex = |i, cpu| {
                cpu.register(i.dest())?.dec();
                Ok(())
            };
            return Ok(i);
        }
        // add hl, r16
        0b1001 => {
            i.op = Operation::ADD;
            let r = R16::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.dest = Some(R16::HL.to_operand());
            i.src = Some(r.to_operand());
            i.length = 1;
            i.cycles = 8;
            i.ex = |i, cpu| {
                let r16 = cpu.register(i.src())?.val();
                let hl_val = cpu.hl_mut().val();
                cpu.flag_reset(Flag::N);
                if bit::add_overflow_u16(r16, hl_val, 11) {
                    cpu.flag_set(Flag::HC);
                }
                if bit::add_overflow_u16(r16, hl_val, 15) {
                    cpu.flag_set(Flag::C);
                }
                cpu.hl_mut().write(r16 + hl_val);
                Ok(())
            };
            return Ok(i);
        }
        _ => (),
    }

    let last_three = opcode & 0b111;
    match last_three {
        // inc r8
        0b100 => {
            i.op = Operation::INC;
            let r = R8::from_u8((opcode & 0b0011_1000) >> 3)?;
            i.dest = Some(r.into());
            i.length = 1;
            i.cycles = 4;
            i.ex = |i, cpu| {
                let prev = cpu.read_byte(i.dest())?;
                let result = cpu.inc_byte(i.dest())?;
                cpu.flag_reset(Flag::N);
                if result == 0 {
                    cpu.flag_reset(Flag::Z);
                }
                if bit::add_overflow(prev, 1, 3) {
                    cpu.flag_set(Flag::HC);
                }
                Ok(())
            };
            return Ok(i);
        }

        // dec r8
        0b101 => {
            i.op = Operation::DEC;
            let r = R8::from_u8((opcode & 0b0011_1000) >> 3)?;
            i.dest = Some(r.into());
            i.length = 1;
            i.cycles = 4;
            i.ex = |i, cpu| {
                let dest = i.dest();
                let prev = cpu.read_byte(i.dest())?;

                let result = cpu.inc_byte(i.dest())?;

                cpu.flag_set(Flag::N);
                if result == 0 {
                    cpu.flag_set(Flag::Z);
                }
                if bit::sub_borrow(prev, 1, 4) {
                    cpu.flag_set(Flag::HC);
                }
                Ok(())
            };
            return Ok(i);
        }

        // ld r8, imm8
        0b110 => {
            i.op = Operation::LD;
            let r = R8::from_u8((opcode & 0b0011_1000) >> 3)?;
            i.dest = Some(r.into());
            i.src = Some(Operand::Imm8);
            i.length = 2;
            match r {
                R8::HlMem => i.cycles = 12,
                _ => i.cycles = 4,
            }
            i.ex = |i, cpu| {
                let imm8 = cpu.fetch()?;
                let r8 = i.src();
                cpu.write_byte(r8, imm8)?;
                Ok(())
            };
            return Ok(i);
        }

        0b000 => {
            let bits_43 = (opcode & 0b0001_1000) >> 3;
            match bits_43 {
                // jr imm8
                0b11 => {
                    i.op = Operation::JR;
                    i.dest = Some(Operand::Imm8);
                    i.length = 2;
                    i.cycles = 12;
                    i.ex = |i, cpu| {
                        let imm8 = cpu.fetch()?;
                        cpu.jump_relative(imm8)?;
                        Ok(())
                    };
                    return Ok(i);
                }
                // jr cond, imm8
                _ => {
                    i.op = Operation::JR;
                    let c = Cond::from_u8(bits_43)?;
                    i.dest = Some(Operand::Cond(c));
                    i.src = Some(Operand::Imm8);
                    i.length = 2;
                    i.cycles = 8;
                    i.branch_cycles = 12;
                    i.ex = |i, cpu| {
                        if cpu.cc(i.src())? {
                            let imm8 = cpu.fetch()?;
                            cpu.jump_relative(imm8)?;
                        }
                        Ok(())
                    };
                    return Ok(i);
                }
            }
        }
        _ => (),
    }

    match opcode {
        // rlca
        0b0000_0111 => {
            i.op = Operation::RLCA;
            i.length = 1;
            i.cycles = 4;
            i.ex = |i, cpu| {
                cpu.clear_flags();
                let a = cpu.read_byte(Some(Operand::A))?;
                let a7 = bit::get(a, 7);
                cpu.flag_set_val(Flag::C, a7);
                let a = (a << 1) + a7;
                cpu.write_byte(Some(Operand::A), a)?;
                Ok(())
            };
            return Ok(i);
        }
        // rrca
        0b0000_1111 => {
            i.op = Operation::RRCA;
            i.length = 1;
            i.cycles = 4;
            i.ex = |i, cpu| {
                cpu.clear_flags();
                let a = cpu.read_byte(Some(Operand::A))?;
                let a0 = bit::get(a, 0);
                cpu.flag_set_val(Flag::C, a0);
                let a = (a >> 1) + (a0 << 7);
                cpu.write_byte(Some(Operand::A), a)?;
                Ok(())
            };
            return Ok(i);
        }
        // rla
        0b0001_0111 => {
            i.op = Operation::RLA;
            i.length = 1;
            i.cycles = 4;
            i.ex = |i, cpu| {
                cpu.clear_flags();
                let a = cpu.read_byte(Some(Operand::A))?;
                let cf = cpu.flag(Flag::C);
                let a7 = bit::get(a, 7);
                cpu.flag_set_val(Flag::C, a7);
                let a = (a << 1) + cf;
                cpu.write_byte(Some(Operand::A), a)?;
                Ok(())
            };
            return Ok(i);
        }
        // rra
        0b0001_1111 => {
            i.op = Operation::RRA;
            i.length = 1;
            i.cycles = 4;
            i.ex = |i, cpu| {
                cpu.clear_flags();
                let a = cpu.read_byte(Some(Operand::A))?;
                let cf = cpu.flag(Flag::C);
                let a0 = bit::get(a, 0);
                cpu.flag_set_val(Flag::C, a0);
                let a = (a >> 1) + (cf << 7);
                cpu.write_byte(Some(Operand::A), a)?;
                Ok(())
            };
            return Ok(i);
        }
        // daa
        0b0010_0111 => {
            i.op = Operation::DAA;
            i.length = 1;
            i.cycles = 4;
            i.ex = |i, cpu| {
                let a = cpu.read_byte(Some(Operand::A))?;
                if cpu.flag_is_set(Flag::N) {
                    let mut adj = 0;
                    if cpu.flag_is_set(Flag::HC) {
                        adj += 0x6;
                    }
                    if cpu.flag_is_set(Flag::C) {
                        adj += 0x60;
                    }
                    let result = a - adj;
                    if result == 0 {
                        cpu.flag_set(Flag::Z);
                    }
                    cpu.write_byte(Some(Operand::A), result)?;
                } else {
                    let mut adj = 0;
                    if cpu.flag_is_set(Flag::HC) || (a & 0xF) > 0x9 {
                        adj += 0x6;
                    }
                    if cpu.flag_is_set(Flag::C) || a > 0x99 {
                        adj += 0x60;
                        cpu.flag_set(Flag::C);
                    }
                    let result = a + adj;
                    if result == 0 {
                        cpu.flag_set(Flag::Z);
                    }
                    cpu.write_byte(Some(Operand::A), result)?;
                }
                Ok(())
            };
            return Ok(i);
        }
        // cpl
        0b0010_1111 => {
            i.op = Operation::CPL;
            i.length = 1;
            i.cycles = 4;
            i.ex = |i, cpu| {
                let a = cpu.read_byte(Some(Operand::A))?;
                cpu.write_byte(Some(Operand::A), !a)?;
                cpu.flag_set(Flag::N);
                cpu.flag_set(Flag::HC);
                Ok(())
            };
            return Ok(i);
        }
        // scf
        0b0011_0111 => {
            i.op = Operation::SCF;
            i.length = 1;
            i.cycles = 4;
            i.ex = |i, cpu| {
                cpu.flag_reset(Flag::N);
                cpu.flag_reset(Flag::HC);
                cpu.flag_set(Flag::C);
                Ok(())
            };
            return Ok(i);
        }
        // ccf
        0b0011_1111 => {
            i.op = Operation::CCF;
            i.length = 1;
            i.cycles = 4;
            i.ex = |i, cpu| {
                cpu.flag_reset(Flag::N);
                cpu.flag_reset(Flag::HC);
                cpu.flag_invert(Flag::C);
                Ok(())
            };
            return Ok(i);
        }
        _ => (),
    }

    Err(InstructionError::Unimplemented(opcode))
}

fn decode_block_1(opcode: u8) -> Result<Instruction, InstructionError> {
    if opcode == 0b0111_0110 {
        return Ok(Instruction::halt());
    } else {
        // ld r8, r8
        let dest = (opcode & 0b0011_1000) >> 4;
        let dest = R8::from_u8(dest)?;
        let src = (opcode & 0b0000_0111) >> 4;
        let src = R8::from_u8(src)?;

        let mut i = Instruction::new();
        i.op = Operation::LD;
        i.dest = Some(dest.into());
        i.src = Some(src.into());

        i.length = 1;
        if dest == R8::HlMem || src == R8::HlMem {
            i.cycles = 8;
        } else {
            i.cycles = 4;
        }

        return Ok(i);
    }
}

fn decode_block_2(opcode: u8) -> Result<Instruction, InstructionError> {
    let mut i = Instruction::new();
    let operand = R8::from_u8(opcode & 0b0000_0111)?;
    i.dest = Some(R8::A.into());
    i.src = Some(operand.into());

    i.length = 1;
    if operand == R8::HlMem {
        i.cycles = 8;
    } else {
        i.cycles = 4;
    }

    match (opcode & 0b1111_1000) >> 3 {
        // add a, r8
        0b1_0000 => {
            i.op = Operation::ADD;
            i.ex = |i, cpu| {
                let a = cpu.read_byte(i.dest())?;
                let val = cpu.read_byte(i.src())?;
                let result = a + val;
                cpu.write_byte(i.dest(), result)?;
                if result == 0 {
                    cpu.flag_set(Flag::Z);
                }
                cpu.flag_reset(Flag::N);
                if bit::add_overflow(a, val, 3) {
                    cpu.flag_set(Flag::HC);
                }
                if bit::add_overflow(a, val, 7) {
                    cpu.flag_set(Flag::C);
                }
                Ok(())
            };
            return Ok(i);
        }
        // adc a, r8
        0b1_0001 => {
            i.op = Operation::ADC;
            i.ex = |i, cpu| {
                let a = cpu.read_byte(i.dest())?;
                let cf = cpu.flag(Flag::C);
                let val = cpu.read_byte(i.src())?;
                let result = a + val + cf;
                cpu.write_byte(i.dest(), result)?;
                if result == 0 {
                    cpu.flag_set(Flag::Z);
                }
                cpu.flag_reset(Flag::N);
                if bit::add_overflow(a, val + cf, 3) {
                    cpu.flag_set(Flag::HC);
                }
                if bit::add_overflow(a, val + cf, 7) {
                    cpu.flag_set(Flag::C);
                }
                Ok(())
            };
            return Ok(i);
        }
        // sub a, r8
        0b1_0010 => {
            i.op = Operation::SUB;
            i.ex = |i, cpu| {
                let a = cpu.read_byte(i.dest())?;
                let val = cpu.read_byte(i.src())?;
                let result = a - val;
                cpu.write_byte(i.dest(), result)?;
                if result == 0 {
                    cpu.flag_set(Flag::Z);
                }
                cpu.flag_set(Flag::N);
                if bit::sub_borrow(a, val, 4) {
                    cpu.flag_set(Flag::HC);
                }
                if bit::sub_borrow(a, val, 8) {
                    cpu.flag_set(Flag::C);
                }
                Ok(())
            };
            return Ok(i);
        }
        // sbc a, r8
        0b1_0011 => {
            i.op = Operation::SBC;
            i.ex = |i, cpu| {
                let a = cpu.read_byte(i.dest())?;
                let cf = cpu.flag(Flag::C);
                let val = cpu.read_byte(i.src())?;
                let result = a - (val + cf);
                cpu.write_byte(i.dest(), result)?;
                if result == 0 {
                    cpu.flag_set(Flag::Z);
                }
                cpu.flag_set(Flag::N);
                if bit::sub_borrow(a, val, 4) {
                    cpu.flag_set(Flag::HC);
                }
                if bit::sub_borrow(a, val + cf, 8) {
                    cpu.flag_set(Flag::C);
                }
                Ok(())
            };
            return Ok(i);
        }
        // and a, r8
        0b1_0100 => {
            i.op = Operation::AND;
            i.ex = |i, cpu| {
                let a = cpu.read_byte(i.dest())?;
                let val = cpu.read_byte(i.src())?;
                let result = a & val;
                cpu.write_byte(i.dest(), result)?;
                if result == 0 {
                    cpu.flag_set(Flag::Z);
                }
                cpu.flag_reset(Flag::N);
                cpu.flag_set(Flag::HC);
                cpu.flag_reset(Flag::C);
                Ok(())
            };
            return Ok(i);
        }
        // xor a, r8
        0b1_0101 => {
            i.op = Operation::XOR;
            i.ex = |i, cpu| {
                let a = cpu.read_byte(i.dest())?;
                let val = cpu.read_byte(i.src())?;
                let result = a ^ val;
                cpu.write_byte(i.dest(), result)?;
                if result == 0 {
                    cpu.flag_set(Flag::Z);
                }
                cpu.flag_reset(Flag::N);
                cpu.flag_reset(Flag::HC);
                cpu.flag_reset(Flag::C);
                Ok(())
            };
            return Ok(i);
        }
        // or a, r8
        0b1_0110 => {
            i.op = Operation::OR;
            i.ex = |i, cpu| {
                let a = cpu.read_byte(i.dest())?;
                let val = cpu.read_byte(i.src())?;
                let result = a | val;
                cpu.write_byte(i.dest(), result)?;
                if result == 0 {
                    cpu.flag_set(Flag::Z);
                }
                cpu.flag_reset(Flag::N);
                cpu.flag_reset(Flag::HC);
                cpu.flag_reset(Flag::C);
                Ok(())
            };
            return Ok(i);
        }
        // cp a, r8
        0b1_0111 => {
            i.op = Operation::CP;
            i.ex = |i, cpu| {
                let a = cpu.read_byte(i.dest())?;
                let val = cpu.read_byte(i.src())?;
                let result = a - val;
                if result == 0 {
                    cpu.flag_set(Flag::Z);
                }
                cpu.flag_set(Flag::N);
                if bit::sub_borrow(a, val, 4) {
                    cpu.flag_set(Flag::HC);
                }
                if bit::sub_borrow(a, val, 8) {
                    cpu.flag_set(Flag::C);
                }
                Ok(())
            };
            return Ok(i);
        }
        _ => (),
    }

    Err(InstructionError::Unimplemented(opcode))
}

const INVALID_OPCODES: [u8; 11] = [
    0xD3, 0xDB, 0xDD, 0xE3, 0xE4, 0xEB, 0xEC, 0xED, 0xF4, 0xFC, 0xFD,
];

fn decode_block_3(opcode: u8) -> Result<Instruction, InstructionError> {
    if INVALID_OPCODES.contains(&opcode) {
        return Err(InstructionError::InvalidOpCode(opcode));
    }

    let mut i = Instruction::new();
    // Prefix
    if opcode == 0b1100_1011 {
        i.op = Operation::PREFIX;
        i.length = 1;
        i.cycles = 4;
        return Ok(i);
    }

    if (opcode & 0b111) == 0b110 {
        i.dest = Some(R8::A.into());
        i.src = Some(Operand::Imm8);
        i.length = 2;
        i.cycles = 8;

        match (opcode & 0b0011_1000) >> 3 {
            // add a, imm8
            0 => {
                i.op = Operation::ADD;
                i.ex = |i, cpu| {
                    let a = cpu.read_byte(i.dest())?;
                    let val = cpu.fetch()?;
                    let result = a + val;
                    cpu.write_byte(i.dest(), result)?;
                    if result == 0 {
                        cpu.flag_set(Flag::Z);
                    }
                    cpu.flag_reset(Flag::N);
                    if bit::add_overflow(a, val, 3) {
                        cpu.flag_set(Flag::HC);
                    }
                    if bit::add_overflow(a, val, 7) {
                        cpu.flag_set(Flag::C);
                    }
                    Ok(())
                };
                return Ok(i);
            }
            // adc a, imm8
            0b1 => {
                i.op = Operation::ADC;
                i.ex = |i, cpu| {
                    let a = cpu.read_byte(i.dest())?;
                    let cf = cpu.flag(Flag::C);
                    let val = cpu.fetch()?;
                    let result = a + val + cf;
                    cpu.write_byte(i.dest(), result)?;
                    if result == 0 {
                        cpu.flag_set(Flag::Z);
                    }
                    cpu.flag_reset(Flag::N);
                    if bit::add_overflow(a, val + cf, 3) {
                        cpu.flag_set(Flag::HC);
                    }
                    if bit::add_overflow(a, val + cf, 7) {
                        cpu.flag_set(Flag::C);
                    }
                    Ok(())
                };
                return Ok(i);
            }
            // sub a, imm8
            0b10 => {
                i.op = Operation::SUB;
                i.ex = |i, cpu| {
                    let a = cpu.read_byte(i.dest())?;
                    let val = cpu.fetch()?;
                    let result = a - val;
                    cpu.write_byte(i.dest(), result)?;
                    if result == 0 {
                        cpu.flag_set(Flag::Z);
                    }
                    cpu.flag_set(Flag::N);
                    if bit::sub_borrow(a, val, 4) {
                        cpu.flag_set(Flag::HC);
                    }
                    if bit::sub_borrow(a, val, 8) {
                        cpu.flag_set(Flag::C);
                    }
                    Ok(())
                };
                return Ok(i);
            }
            // sbc a, imm8
            0b11 => {
                i.op = Operation::SBC;
                i.ex = |i, cpu| {
                    let a = cpu.read_byte(i.dest())?;
                    let cf = cpu.flag(Flag::C);
                    let val = cpu.fetch()?;
                    let result = a - (val + cf);
                    cpu.write_byte(i.dest(), result)?;
                    if result == 0 {
                        cpu.flag_set(Flag::Z);
                    }
                    cpu.flag_set(Flag::N);
                    if bit::sub_borrow(a, val, 4) {
                        cpu.flag_set(Flag::HC);
                    }
                    if bit::sub_borrow(a, val + cf, 8) {
                        cpu.flag_set(Flag::C);
                    }
                    Ok(())
                };
                return Ok(i);
            }
            // and a, imm8
            0b100 => {
                i.op = Operation::AND;
                i.ex = |i, cpu| {
                    let a = cpu.read_byte(i.dest())?;
                    let val = cpu.fetch()?;
                    let result = a & val;
                    cpu.write_byte(i.dest(), result)?;
                    if result == 0 {
                        cpu.flag_set(Flag::Z);
                    }
                    cpu.flag_reset(Flag::N);
                    cpu.flag_set(Flag::HC);
                    cpu.flag_reset(Flag::C);
                    Ok(())
                };
                return Ok(i);
            }
            // xor a, imm8
            0b101 => {
                i.op = Operation::XOR;
                i.ex = |i, cpu| {
                    let a = cpu.read_byte(i.dest())?;
                    let val = cpu.fetch()?;
                    let result = a ^ val;
                    cpu.write_byte(i.dest(), result)?;
                    if result == 0 {
                        cpu.flag_set(Flag::Z);
                    }
                    cpu.flag_reset(Flag::N);
                    cpu.flag_reset(Flag::HC);
                    cpu.flag_reset(Flag::C);
                    Ok(())
                };
                return Ok(i);
            }
            // or a, imm8
            0b110 => {
                i.op = Operation::OR;
                i.ex = |i, cpu| {
                    let a = cpu.read_byte(i.dest())?;
                    let val = cpu.fetch()?;
                    let result = a | val;
                    cpu.write_byte(i.dest(), result)?;
                    if result == 0 {
                        cpu.flag_set(Flag::Z);
                    }
                    cpu.flag_reset(Flag::N);
                    cpu.flag_reset(Flag::HC);
                    cpu.flag_reset(Flag::C);
                    Ok(())
                };
                return Ok(i);
            }
            // cp a, imm8
            0b111 => {
                i.op = Operation::CP;
                i.ex = |i, cpu| {
                    let a = cpu.read_byte(i.dest())?;
                    let val = cpu.fetch()?;
                    let result = a - val;
                    if result == 0 {
                        cpu.flag_set(Flag::Z);
                    }
                    cpu.flag_set(Flag::N);
                    if bit::sub_borrow(a, val, 4) {
                        cpu.flag_set(Flag::HC);
                    }
                    if bit::sub_borrow(a, val, 8) {
                        cpu.flag_set(Flag::C);
                    }
                    Ok(())
                };
                return Ok(i);
            }
            _ => (),
        }

        match opcode {
            // ldh [c], a
            0b1110_0010 => {
                i.op = Operation::LDH;
                i.dest = Some(Operand::C.to_mem());
                i.src = Some(Operand::A);
                i.length = 1;
                i.cycles = 8;
                i.ex = |i, cpu| {
                    let c = cpu.read_byte(i.dest())? as u16;
                    let addr = c + 0xFF00;
                    let a = cpu.read_byte(i.src())?;
                    let mu = cpu.mem();
                    let mut mem = mu.lock().unwrap();
                    mem.write(addr, a)?;
                    Ok(())
                };
                return Ok(i);
            }

            // ldh [imm8], a
            0b1110_0000 => {
                i.op = Operation::LD;
                i.dest = Some(Operand::Imm8.to_mem());
                i.src = Some(Operand::A);
                i.length = 2;
                i.cycles = 12;
                i.ex = |i, cpu| {
                    let imm8 = cpu.fetch()? as u16;
                    let addr = imm8 + 0xFF00;
                    let a = cpu.read_byte(i.src())?;
                    let mu = cpu.mem();
                    let mut mem = mu.lock().unwrap();
                    mem.write(addr, a)?;
                    Ok(())
                };
                return Ok(i);
            }

            // ld [imm16], a
            0b1110_1010 => {
                i.op = Operation::LD;
                i.dest = Some(Operand::Imm16.to_mem());
                i.src = Some(Operand::A);
                i.length = 3;
                i.cycles = 16;
                i.ex = |i, cpu| {
                    let imm16 = cpu.fetch_word()?;
                    let a = cpu.read_byte(i.src())?;
                    let mu = cpu.mem();
                    let mut mem = mu.lock().unwrap();
                    mem.write(imm16, a)?;
                    Ok(())
                };
                return Ok(i);
            }

            // ldh a, [c]
            0b1111_0010 => {
                i.op = Operation::LDH;
                i.dest = Some(Operand::A);
                i.src = Some(Operand::C.to_mem());
                i.length = 1;
                i.cycles = 8;
                i.ex = |i, cpu| {
                    let c = cpu.read_byte(i.dest())? as u16;
                    let addr = c + 0xFF00;
                    let mu = cpu.mem();
                    let mem = mu.lock().unwrap();
                    let val = mem.read(addr)?;
                    cpu.write_byte(i.dest(), val)?;
                    Ok(())
                };
                return Ok(i);
            }

            // ldh a, [imm8]
            0b1111_0000 => {
                i.op = Operation::LDH;
                i.dest = Some(Operand::A);
                i.src = Some(Operand::Imm8.to_mem());
                i.length = 2;
                i.cycles = 12;
                i.ex = |i, cpu| {
                    let imm8 = cpu.fetch()? as u16;
                    let addr = imm8 + 0xFF00;
                    let mu = cpu.mem();
                    let mem = mu.lock().unwrap();
                    let val = mem.read(addr)?;
                    cpu.write_byte(i.dest(), val)?;
                    Ok(())
                };
                return Ok(i);
            }

            // ld a, [imm16]
            0b1111_1010 => {
                i.op = Operation::LD;
                i.dest = Some(Operand::A);
                i.src = Some(Operand::Imm16.to_mem());
                i.length = 3;
                i.cycles = 16;
                i.ex = |i, cpu| {
                    let imm16 = cpu.fetch_word()?;
                    let mu = cpu.mem();
                    let mem = mu.lock().unwrap();
                    let val = mem.read(imm16)?;
                    cpu.write_byte(i.dest(), val)?;
                    Ok(())
                };
                return Ok(i);
            }

            // add sp, imm8
            0b1110_1000 => {
                i.op = Operation::ADD;
                i.dest = Some(R16::SP.to_operand());
                i.src = Some(Operand::Imm8);
                i.length = 2;
                i.cycles = 16;
                i.ex = |i, cpu| {
                    let sp = cpu.read(i.dest())?;
                    let imm8 = cpu.fetch()? as u16;
                    let result = sp + imm8;
                    cpu.write(i.dest(), result)?;
                    cpu.flag_reset(Flag::Z);
                    cpu.flag_reset(Flag::N);
                    if bit::add_overflow_u16(sp, imm8, 3) {
                        cpu.flag_set(Flag::HC);
                    }
                    if bit::add_overflow_u16(sp, imm8, 7) {
                        cpu.flag_set(Flag::C);
                    }
                    Ok(())
                };
                return Ok(i);
            }

            // ld hl, sp + imm8
            0b1111_1000 => {
                i.op = Operation::LD;
                i.dest = Some(R16::HL.to_operand());
                let sp = Box::new(R16::SP.to_operand());
                let imm8 = Box::new(Operand::Imm8);
                i.src = Some(Operand::Sum((sp, imm8)));
                i.length = 2;
                i.cycles = 12;
                i.ex = |i, cpu| {
                    let sp = cpu.read(i.dest())?;
                    let imm8 = cpu.fetch()? as u16;
                    let result = sp + imm8;
                    cpu.write(Some(Operand::HL), result)?;

                    cpu.flag_reset(Flag::Z);
                    cpu.flag_reset(Flag::N);
                    if bit::add_overflow_u16(sp, imm8, 3) {
                        cpu.flag_set(Flag::HC);
                    }
                    if bit::add_overflow_u16(sp, imm8, 7) {
                        cpu.flag_set(Flag::C);
                    }
                    Ok(())
                };
                return Ok(i);
            }

            // ld sp, hl
            0b1111_1001 => {
                i.op = Operation::LD;
                i.dest = Some(R16::SP.to_operand());
                i.src = Some(R16::HL.to_operand());
                i.length = 1;
                i.cycles = 8;
                i.ex = |i, cpu| {
                    let hl = cpu.read(i.src())?;
                    cpu.write(i.dest(), hl)?;
                    Ok(())
                };
                return Ok(i);
            }

            // di
            0b1111_0011 => {
                i.op = Operation::DI;
                i.length = 1;
                i.cycles = 4;
                i.ex = |_, cpu| {
                    cpu.disable_interrupts();
                    Ok(())
                };
                return Ok(i);
            }

            // ei
            0b1111_1011 => {
                i.op = Operation::EI;
                i.length = 1;
                i.cycles = 4;
                i.ex = |_, cpu| {
                    cpu.enable_interrupts();
                    Ok(())
                };
                return Ok(i);
            }

            // ret
            0b1100_1001 => {
                i.op = Operation::RET;
                i.length = 1;
                i.cycles = 16;
                i.ex = |_, cpu| {
                    let sp = cpu.pop_stack()?;
                    cpu.set_pc(sp);
                    Ok(())
                };
                return Ok(i);
            }
            // reti
            0b1101_1001 => {
                i.op = Operation::RET;
                i.length = 1;
                i.cycles = 16;
                i.ex = |_, cpu| {
                    let sp_val = cpu.pop_stack()?;
                    cpu.set_pc(sp_val);
                    cpu.enable_interrupts();
                    Ok(())
                };
            }

            // jp imm16
            0b1100_0011 => {
                i.op = Operation::JP;
                i.dest = Some(Operand::Imm16);
                i.length = 3;
                i.cycles = 16;
                i.ex = |_, cpu| {
                    let imm16 = cpu.fetch_word()?;
                    cpu.set_pc(imm16);
                    Ok(())
                };
                return Ok(i);
            }
            // jp hl
            0b1110_1001 => {
                i.op = Operation::JP;
                i.dest = Some(Operand::HL);
                i.length = 3;
                i.cycles = 16;
                i.ex = |i, cpu| {
                    let hl = cpu.read(i.dest())?;
                    cpu.set_pc(hl);
                    Ok(())
                };
                return Ok(i);
            }

            // call imm16
            0b1100_1101 => {
                i.op = Operation::CALL;
                i.dest = Some(Operand::Imm16);
                i.length = 3;
                i.cycles = 24;
                i.ex = |_, cpu| {
                    let imm16 = cpu.fetch_word()?;
                    let pc = cpu.get_pc();
                    cpu.push_stack(pc)?;
                    cpu.set_pc(imm16);
                    Ok(())
                };
                return Ok(i);
            }
            _ => (),
        }
    }

    let last_three = opcode & 0b111;
    match last_three {
        // ret cond
        0b000 => {
            i.op = Operation::RET;
            let c = Cond::from_u8((opcode & 0b0001_1000) >> 3)?;
            i.dest = Some(c.to_operand());
            i.length = 1;
            i.cycles = 8;
            i.branch_cycles = 20;
            i.ex = |i, cpu| {
                if cpu.cc(i.dest())? {
                    let sp = cpu.read(Some(Operand::SP))?;
                    cpu.set_pc(sp);
                    let sp = sp + 2;
                    cpu.write(Some(Operand::SP), sp)?;
                }
                Ok(())
            };
            return Ok(i);
        }
        // jp cond, imm16
        0b010 => {
            i.op = Operation::JP;
            let c = Cond::from_u8((opcode & 0b0001_1000) >> 3)?;
            i.dest = Some(c.to_operand());
            i.src = Some(Operand::Imm16);
            i.length = 3;
            i.cycles = 12;
            i.branch_cycles = 16;
            i.ex = |i, cpu| {
                if cpu.cc(i.dest())? {
                    let imm16 = cpu.fetch_word()?;
                    cpu.set_pc(imm16);
                }
                Ok(())
            };
            return Ok(i);
        }
        // call cond, imm16
        0b100 => {
            i.op = Operation::CALL;
            let c = Cond::from_u8((opcode & 0b0001_1000) >> 3)?;
            i.dest = Some(c.to_operand());
            i.src = Some(Operand::Imm16);
            i.length = 3;
            i.cycles = 12;
            i.branch_cycles = 24;
            i.ex = |i, cpu| {
                if cpu.cc(i.dest())? {
                    let imm16 = cpu.fetch_word()?;
                    let pc = cpu.get_pc();
                    cpu.push_stack(pc)?;
                    cpu.set_pc(imm16);
                }
                Ok(())
            };
            return Ok(i);
        }
        // rst tgt3
        0b111 => {
            i.op = Operation::RST;
            let t = Tgt3::new((opcode & 0b0011_1000) >> 3);
            i.dest = Some(Operand::Tgt3(t));
            i.length = 1;
            i.cycles = 16;
            i.ex = |i, cpu| {
                let t: Tgt3 = i.dest().into();
                let t = t.addr();
                let pc = cpu.get_pc();
                cpu.push_stack(pc)?;
                cpu.set_pc(t as u16);
                Ok(())
            };
            return Ok(i);
        }
        _ => (),
    }

    let last_four = opcode & 0b1111;
    match last_four {
        // pop r16stk
        0b0001 => {
            i.op = Operation::POP;
            let r = R16Stk::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.dest = Some(r.to_operand());
            i.length = 1;
            i.cycles = 12;
            i.ex = |i, cpu| {
                let sp_val = cpu.pop_stack()?;
                cpu.write(i.dest(), sp_val)?;
                Ok(())
            };
            return Ok(i);
        }
        // push r16stk
        0b0101 => {
            i.op = Operation::PUSH;
            let r = R16Stk::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.dest = Some(r.to_operand());
            i.length = 1;
            i.cycles = 16;
            i.ex = |i, cpu| {
                let r16_val = cpu.read(i.dest())?;
                cpu.push_stack(r16_val)?;
                Ok(())
            };
            return Ok(i);
        }
        _ => (),
    }

    Err(InstructionError::Unknown)
}

fn decode_prefix(opcode: u8) -> Result<Instruction, InstructionError> {
    let mut i = Instruction::new();

    let first_five = (opcode & 0b1111_1000) >> 3;
    match first_five {
        // rlc r8
        0b000 => {
            i.op = Operation::RLC;
            let r = R8::from_u8(opcode & 0b111)?;
            i.dest = Some(r.into());
            i.length = 2;
            if r == R8::HlMem {
                i.cycles = 16;
            } else {
                i.cycles = 8;
            }
            return Ok(i);
        }
        // rrc r8
        0b001 => {
            i.op = Operation::RRC;
            let r = R8::from_u8(opcode & 0b111)?;
            i.dest = Some(r.into());
            i.length = 2;
            if r == R8::HlMem {
                i.cycles = 16;
            } else {
                i.cycles = 8;
            }
            return Ok(i);
        }
        // rl r8
        0b010 => {
            i.op = Operation::RL;
            let r = R8::from_u8(opcode & 0b111)?;
            i.dest = Some(r.into());
            i.length = 2;
            if r == R8::HlMem {
                i.cycles = 16;
            } else {
                i.cycles = 8;
            }
            return Ok(i);
        }
        // rr r8
        0b011 => {
            i.op = Operation::RR;
            let r = R8::from_u8(opcode & 0b111)?;
            i.dest = Some(r.into());
            i.length = 2;
            if r == R8::HlMem {
                i.cycles = 16;
            } else {
                i.cycles = 8;
            }
            return Ok(i);
        }
        // sla r8
        0b100 => {
            i.op = Operation::SLA;
            let r = R8::from_u8(opcode & 0b111)?;
            i.dest = Some(r.into());
            i.length = 2;
            if r == R8::HlMem {
                i.cycles = 16;
            } else {
                i.cycles = 8;
            }
            return Ok(i);
        }
        // sra r8
        0b101 => {
            i.op = Operation::SRA;
            let r = R8::from_u8(opcode & 0b111)?;
            i.dest = Some(r.into());
            i.length = 2;
            if r == R8::HlMem {
                i.cycles = 16;
            } else {
                i.cycles = 8;
            }
            return Ok(i);
        }
        // swap r8
        0b110 => {
            i.op = Operation::RL;
            let r = R8::from_u8(opcode & 0b111)?;
            i.dest = Some(r.into());
            i.length = 2;
            if r == R8::HlMem {
                i.cycles = 16;
            } else {
                i.cycles = 8;
            }
            return Ok(i);
        }
        // srl r8
        0b111 => {
            i.op = Operation::SRL;
            let r = R8::from_u8(opcode & 0b111)?;
            i.dest = Some(r.into());
            i.length = 2;
            if r == R8::HlMem {
                i.cycles = 16;
            } else {
                i.cycles = 8;
            }
            return Ok(i);
        }
        _ => (),
    }

    let last_two = (opcode & 0b1100_0000) >> 6;
    match last_two {
        // bit b3, r8
        0b01 => {
            i.op = Operation::BIT;
            let b3 = BitIndex::new((opcode & 0b11_1000) >> 3);
            let r = R8::from_u8(opcode & 0b111)?;
            i.dest = Some(b3.to_operand());
            i.src = Some(r.into());
            i.length = 2;
            if r == R8::HlMem {
                i.cycles = 16;
            } else {
                i.cycles = 8;
            }
            return Ok(i);
        }
        // res b3, r8
        0b10 => {
            i.op = Operation::BIT;
            let b3 = BitIndex::new((opcode & 0b11_1000) >> 3);
            let r = R8::from_u8(opcode & 0b111)?;
            i.dest = Some(b3.to_operand());
            i.src = Some(b3.to_operand());
            i.length = 2;
            if r == R8::HlMem {
                i.cycles = 16;
            } else {
                i.cycles = 8;
            }
            return Ok(i);
        }
        // set b3, r8
        0b11 => {
            i.op = Operation::BIT;
            let b3 = BitIndex::new((opcode & 0b11_1000) >> 3);
            let r = R8::from_u8(opcode & 0b111)?;
            i.dest = Some(b3.to_operand());
            i.src = Some(r.into());
            i.length = 2;
            if r == R8::HlMem {
                i.cycles = 16;
            } else {
                i.cycles = 8;
            }
            return Ok(i);
        }
        _ => (),
    }

    Err(InstructionError::Unknown)
}

#[cfg(test)]
mod test {
    use crate::instructions::{Cond, Operand, Operation, R16, decode};

    // BLOCK 0 TESTS
    #[test]
    fn decode_nop() {
        let i = decode(0).unwrap();
        assert_eq!(i.op, Operation::NOP);
        assert_eq!(i.dest, None);
        assert_eq!(i.src, None);
        assert_eq!(i.length, 1);
        assert_eq!(i.cycles, 4);
    }

    fn decode_stop() {
        let i = decode(0x10).unwrap();
        assert_eq!(i.op, Operation::STOP);
        assert_eq!(i.dest, None);
        assert_eq!(i.src, None);
        assert_eq!(i.length, 1);
        assert_eq!(i.cycles, 4);
    }

    #[test]
    fn decode_ld_r16_imm16() {
        let opcode = 0b0001_0001;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::LD);
        assert_eq!(i.dest, Some(R16::DE.to_operand()));
        assert_eq!(i.src, Some(Operand::Imm16));
        assert_eq!(i.length, 3);
        assert_eq!(i.cycles, 12);
    }

    #[test]
    fn decode_ld_r16mem_a() {
        let opcode = 0b0001_0010;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::LD);
        assert_eq!(i.dest, Some(Operand::BC.to_mem()));
        assert_eq!(i.src, Some(Operand::A));
        assert_eq!(i.length, 1);
        assert_eq!(i.cycles, 8);
    }

    #[test]
    fn decode_ld_a_r16mem() {
        let opcode = 0b0001_1010;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::LD);
        assert_eq!(i.dest, Some(Operand::A));
        assert_eq!(i.src, Some(Operand::BC.to_mem()));
        assert_eq!(i.length, 1);
        assert_eq!(i.cycles, 8);
    }

    #[test]
    fn decode_ld_imm16mem_sp() {
        let opcode = 0b0000_1000;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::LD);
        assert_eq!(i.dest, Some(Operand::Imm16.to_mem()));
        assert_eq!(i.src, Some(Operand::SP));
        assert_eq!(i.length, 3);
        assert_eq!(i.cycles, 20);
    }

    #[test]
    fn decode_inc_r16() {
        let opcode = 0b0001_0011;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::INC);
        assert_eq!(i.dest, Some(Operand::DE));
        assert_eq!(i.src, None);
        assert_eq!(i.length, 1);
        assert_eq!(i.cycles, 8);
    }

    #[test]
    fn decode_dec_r16() {
        let opcode = 0b0001_1011;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::DEC);
        assert_eq!(i.dest, Some(Operand::DE));
        assert_eq!(i.src, None);
        assert_eq!(i.length, 1);
        assert_eq!(i.cycles, 8);
    }

    #[test]
    fn decode_add_hl_r16() {
        let opcode = 0x09;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::ADD);
        assert_eq!(i.dest, Some(Operand::HL));
        assert_eq!(i.src, Some(Operand::BC));
    }

    #[test]
    fn decode_inc_r8() {
        let opcode = 0x04;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::INC);
        assert_eq!(i.dest, Some(Operand::B));
        assert_eq!(i.src, None);
    }

    #[test]
    fn decode_dec_r8() {
        let opcode = 0x05;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::DEC);
        assert_eq!(i.dest, Some(Operand::B));
        assert_eq!(i.src, None);
    }

    #[test]
    fn decode_ld_r8_imm8() {
        let opcode = 0x36;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::LD);
        assert_eq!(i.dest, Some(Operand::HL.to_mem()));
        assert_eq!(i.src, Some(Operand::Imm8));
    }

    #[test]
    fn decode_jr_imm8() {
        let opcode = 0x18;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::JR);
        assert_eq!(i.dest, Some(Operand::Imm8));
    }

    #[test]
    fn decode_jr_cond_imm8() {
        let opcode = 0x28;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::JR);
        assert_eq!(i.dest, Some(Operand::Cond(Cond::Z)));
        assert_eq!(i.src, Some(Operand::Imm8));
    }
}
