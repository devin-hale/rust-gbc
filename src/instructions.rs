use std::fmt::Display;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Decoding error: {0}")]
    Decode(DecodeError),

    #[error("unknown")]
    Unknown,
}

impl From<DecodeError> for Error {
    fn from(value: DecodeError) -> Self {
        Error::Decode(value)
    }
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("'0x{0:x}' is not a valid R8 value")]
    InvalidR8(u8),

    #[error("'0x{0:x}' is not a valid R16 value")]
    InvalidR16(u8),

    #[error("'0x{0:x}' is not a valid R16Stk value")]
    InvalidR16Stk(u8),

    #[error("'0x{0:x}' is not a valid R16Mem value")]
    InvalidR16Mem(u8),

    #[error("'0x{0:x}' is not a valid Cond value")]
    InvalidCond(u8),

    #[error("opcode '0x{0:x}' is unimplemented")]
    Unimplemented(u8),

    #[error("unknown")]
    Unknown,
}

pub struct Instruction {
    opcode: u8,
    op: Operation,
    length: u8,
    cycles: (u8, u8),

    z: FlagBehavior,
    n: FlagBehavior,
    h: FlagBehavior,
    c: FlagBehavior,

    // for purposes for proper string representation when executing
    n8: Option<u8>,
    n16: Option<u16>,
}

impl Instruction {
    pub fn new() -> Instruction {
        Instruction::default()
    }

    pub fn cycles(&self) -> u8 {
        self.cycles.0
    }

    pub fn branch_cycles(&self) -> u8 {
        self.cycles.1
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = self.op.to_string();
        if let Some(n) = self.n8 {
            s = s.replace("n8", n.to_string().as_str());
        }
        if let Some(nn) = self.n8 {
            s = s.replace("n16", nn.to_string().as_str());
        }
        write!(f, "{}", s)
    }
}

impl Default for Instruction {
    fn default() -> Self {
        Instruction {
            opcode: 0,
            op: Operation::NOP,
            length: 1,
            cycles: (4, 0),
            z: Default::default(),
            n: Default::default(),
            h: Default::default(),
            c: Default::default(),
            n8: None,
            n16: None,
        }
    }
}

#[derive(Default)]
pub enum FlagState {
    #[default]
    Reset,
    Set,
}

#[derive(Default)]
pub enum FlagBehavior {
    #[default]
    None,
    Set,
    Reset,
    Overflow(u8),
    Borrow(u8),
    IfZero(FlagState),
    Invert,
    Dependent,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum R8 {
    B,
    C,
    D,
    E,
    H,
    L,
    HL,
    A,
    N8,
}

pub const R8VALUES: [R8; 9] = [
    R8::B,
    R8::C,
    R8::D,
    R8::E,
    R8::H,
    R8::L,
    R8::HL,
    R8::A,
    R8::N8,
];

impl TryFrom<u8> for R8 {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(R8::B),
            1 => Ok(R8::C),
            2 => Ok(R8::D),
            3 => Ok(R8::E),
            4 => Ok(R8::H),
            5 => Ok(R8::L),
            6 => Ok(R8::HL),
            7 => Ok(R8::A),
            _ => Err(DecodeError::InvalidR8(value).into()),
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
            R8::HL => "[hl]",
            R8::A => "a",
            R8::N8 => "n8",
        };
        write!(f, "{}", r8)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mem {
    HL,
    BC,
    DE,
    SP,
    HLI,
    HLD,
    SPN8,
    N16,
    N8,
    C,
}

impl Mem {
    fn r16mem(value: u8) -> Result<Mem, Error> {
        match value {
            0 => Ok(Mem::BC),
            1 => Ok(Mem::DE),
            2 => Ok(Mem::HLI),
            3 => Ok(Mem::HLD),
            _ => Err(DecodeError::InvalidR16Mem(value).into()),
        }
    }
}

impl Display for Mem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Mem::C => "[c]",
            Mem::N16 => "[n16]",
            Mem::N8 => "[n8]",
            Mem::DE => "[de]",
            Mem::BC => "[bc]",
            Mem::SP => "[sp]",
            Mem::HL => "[hl]",
            Mem::HLI => "[hl+]",
            Mem::HLD => "[hl-]",
            Mem::SPN8 => "sp + n8",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum R16 {
    BC,
    DE,
    HL,
    SP,
    AF,
    PC,
    N16,
}

impl R16 {
    fn r16stk(v: u8) -> Result<R16, Error> {
        match v {
            0 => Ok(R16::BC),
            1 => Ok(R16::DE),
            2 => Ok(R16::HL),
            3 => Ok(R16::AF),
            _ => Err(DecodeError::InvalidR16Stk(v).into()),
        }
    }
}

impl TryFrom<u8> for R16 {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(R16::BC),
            1 => Ok(R16::DE),
            2 => Ok(R16::HL),
            3 => Ok(R16::SP),
            _ => Err(DecodeError::InvalidR16(value).into()),
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
            R16::AF => "af",
            R16::PC => "pc",
            R16::N16 => "n16",
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

impl TryFrom<u8> for Cond {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Cond::NZ),
            1 => Ok(Cond::Z),
            2 => Ok(Cond::NC),
            3 => Ok(Cond::C),
            _ => Err(DecodeError::InvalidCond(value).into()),
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
pub struct B3(u8);

impl B3 {
    pub fn val(&self) -> u8 {
        self.0
    }
}

impl From<u8> for B3 {
    fn from(value: u8) -> Self {
        B3(value & 0x3)
    }
}

impl Display for B3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct T3(u8);

impl From<u8> for T3 {
    fn from(value: u8) -> Self {
        T3(value / 8)
    }
}

impl From<T3> for u8 {
    fn from(value: T3) -> Self {
        value.0
    }
}

impl Display for T3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operation {
    NOP,
    LD(LD),
    INC(INC),
    DEC(DEC),
    ADD(ADD),
    RLCA,
    RRCA,
    RLA,
    RRA,
    DAA,
    CPL,
    SCF,
    CCF,
    JR(JR),
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
        let op: String = match self {
            Operation::NOP => String::from("nop"),
            Operation::LD(ld) => ld.to_string(),
            Operation::INC(inc) => inc.to_string(),
            Operation::DEC(dec) => dec.to_string(),
            Operation::ADD(add) => add.to_string(),
            //Operation::RLCA => "rlca",
            //Operation::RRCA => "rrca",
            //Operation::RLA => "rla",
            //Operation::RRA => "rra",
            //Operation::DAA => "daa",
            //Operation::CPL => "cpl",
            //Operation::SCF => "scf",
            //Operation::CCF => "ccf",
            Operation::JR(jr) => jr.to_string(),
            Operation::STOP => String::from("stop"),
            //Operation::HALT => "halt",
            //Operation::ADC => "adc",
            //Operation::SUB => "sub",
            //Operation::SBC => "sbc",
            //Operation::AND => "and",
            //Operation::XOR => "xor",
            //Operation::OR => "or",
            //Operation::CP => "cp",
            //Operation::RET => "ret",
            //Operation::RETI => "reti",
            //Operation::JP => "jp",
            //Operation::CALL => "call",
            //Operation::RST => "rst",
            //Operation::POP => "pop",
            //Operation::PUSH => "push",
            //Operation::LDH => "ldh",
            //Operation::DI => "di",
            //Operation::EI => "ei",
            //Operation::RR => "rr",
            //Operation::RLC => "rlc",
            //Operation::RRC => "rrc",
            //Operation::RL => "rl",
            //Operation::SLA => "sla",
            //Operation::SRA => "sra",
            //Operation::SWAP => "swap",
            //Operation::SRL => "srl",
            //Operation::BIT => "bit",
            //Operation::RES => "res",
            //Operation::SET => "set",
            //Operation::PREFIX => "cb prefix",
            _ => String::from(""),
        };
        write!(f, "{}", op)
    }
}

impl From<ADD> for Operation {
    fn from(value: ADD) -> Self {
        Operation::ADD(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ADD {
    R8(R8, R8),
    R16(R16, R16),
    SP(i8),
}

impl Display for ADD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::from("add");
        match self {
            ADD::R16(a, b) => s.push_str(format!("{}, {}", a, b).as_str()),
            ADD::R8(a, b) => s.push_str(format!("{}, {}", a, b).as_str()),
            ADD::SP(n8) => s.push_str(format!("sp, {}", n8).as_str()),
        }
        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LD {
    R8(R8, R8),
    R16(R16, R16),
    MemR8(Mem, R8),
    R8Mem(R8, Mem),
    MemR16(Mem, R16),
}

impl From<LD> for Operation {
    fn from(value: LD) -> Self {
        Operation::LD(value)
    }
}

impl Display for LD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::from("ld");
        match self {
            LD::R16(a, b) => s.push_str(format!("{}, {}", a, b).as_str()),
            LD::R8(a, b) => s.push_str(format!("{}, {}", a, b).as_str()),
            LD::MemR8(a, b) => s.push_str(format!("{}, {}", a, b).as_str()),
            LD::R8Mem(a, b) => s.push_str(format!("{}, {}", a, b).as_str()),
            LD::MemR16(a, b) => s.push_str(format!("{}, {}", a, b).as_str()),
        }
        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum INC {
    R8(R8),
    R16(R16),
}

impl From<INC> for Operation {
    fn from(value: INC) -> Self {
        Operation::INC(value)
    }
}

impl Display for INC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let o = match self {
            INC::R8(r) => format!("{}", r),
            INC::R16(r) => format!("{}", r),
        };
        write!(f, "inc {}", o)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DEC {
    R8(R8),
    R16(R16),
}

impl From<DEC> for Operation {
    fn from(value: DEC) -> Self {
        Operation::DEC(value)
    }
}

impl Display for DEC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let o = match self {
            DEC::R8(r) => format!("{}", r),
            DEC::R16(r) => format!("{}", r),
        };
        write!(f, "inc {}", o)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JR {
    N8,
    Cond(Cond),
}

impl From<JR> for Operation {
    fn from(value: JR) -> Self {
        Operation::JR(value)
    }
}

impl Display for JR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            JR::N8 => String::from("n8"),
            JR::Cond(c) => format!("{}, n8", c),
        };
        write!(f, "jr {}", s)
    }
}

pub fn decode(opcode: u8) -> Result<Instruction, Error> {
    match (opcode >> 6) & 0x3 {
        0x00 => Ok(decode_block_0(opcode)?),
        //0x01 => Ok(decode_block_1(opcode)?),
        //0x10 => Ok(decode_block_2(opcode)?),
        //0x11 => {
        //    let i = decode_block_3(opcode)?;
        //    if i.op == Operation::PREFIX {
        //        return Ok(decode_prefix(opcode)?);
        //    } else {
        //        return Ok(i);
        //    }
        //}
        _ => Err(Error::Unknown),
    }
}

fn decode_block_0(opcode: u8) -> Result<Instruction, Error> {
    if opcode == 0x00 {
        return Ok(Instruction::default());
    }
    if opcode == 0xF0 {
        let mut i = Instruction::new();
        i.opcode = opcode;
        i.op = Operation::STOP;
        return Ok(i);
    }

    let mut i = Instruction {
        opcode,
        ..Default::default()
    };
    let last_four = opcode & 0b1111;
    match last_four {
        // ld r16, n16
        0b0001 => {
            let r16: R16 = ((opcode & 0b0011_0000) >> 4).try_into()?;
            i.op = LD::R16(r16, R16::N16).into();
            i.cycles = (12, 0);
            i.length = 3;
            return Ok(i);
        }
        //// ld [r16mem], a
        0b0010 => {
            let r16 = Mem::r16mem((opcode & 0b0011_0000) >> 4)?;
            i.op = LD::MemR8(r16, R8::A).into();
            i.length = 1;
            i.cycles = (8, 0);
            return Ok(i);
        }
        // ld a, [r16mem]
        0b1010 => {
            let rm = Mem::r16mem((opcode & 0b0011_0000) >> 4)?;
            i.op = LD::R8Mem(R8::A, rm).into();
            i.length = 1;
            i.cycles = (8, 0);
            return Ok(i);
        }
        // ld [imm16], sp
        0b1000 => {
            let dest = (opcode & 0b0011_0000) >> 4;
            if dest == 0b00 {
                i.op = LD::MemR16(Mem::N16, R16::SP).into();
                i.length = 3;
                i.cycles = (20, 0);
                return Ok(i);
            }
        }

        // inc r16
        0b0011 => {
            let r: R16 = ((opcode & 0b0011_0000) >> 4).try_into()?;
            i.op = INC::R16(r).into();
            i.length = 1;
            i.cycles = (8, 0);
            return Ok(i);
        }

        // dec r16
        0b1011 => {
            let r: R16 = ((opcode & 0b0011_0000) >> 4).try_into()?;
            i.op = DEC::R16(r).into();
            i.length = 1;
            i.cycles = (8, 0);
            return Ok(i);
        }

        // add hl, r16
        0b1001 => {
            let r: R16 = ((opcode & 0b0011_0000) >> 4).try_into()?;
            i.op = ADD::R16(R16::HL, r).into();
            i.length = 1;
            i.cycles = (8, 0);
            i.n = FlagBehavior::Reset;
            i.h = FlagBehavior::Overflow(11);
            i.c = FlagBehavior::Overflow(15);
            return Ok(i);
        }
        _ => (),
    }

    let last_three = opcode & 0b111;
    match last_three {
        // inc r8
        0b100 => {
            let r: R8 = ((opcode & 0b0011_1000) >> 3).try_into()?;
            i.op = INC::R8(r).into();
            i.length = 1;
            i.cycles = (4, 0);
            i.z = FlagBehavior::IfZero(FlagState::Reset);
            i.n = FlagBehavior::Reset;
            i.h = FlagBehavior::Overflow(3);
            return Ok(i);
        }

        // dec r8
        0b101 => {
            let r: R8 = ((opcode & 0b0011_1000) >> 3).try_into()?;
            i.op = DEC::R8(r).into();
            i.length = 1;
            i.cycles = (4, 0);
            i.z = FlagBehavior::IfZero(FlagState::Set);
            i.h = FlagBehavior::Borrow(4);
            i.n = FlagBehavior::Set;
            return Ok(i);
        }

        // ld r8, n8
        0b110 => {
            let r: R8 = ((opcode & 0b0011_1000) >> 3).try_into()?;
            i.op = LD::R8(r, R8::N8).into();
            i.length = 2;
            match r {
                R8::HL => i.cycles = (12, 0),
                _ => i.cycles = (4, 0),
            }
            return Ok(i);
        }

        0b000 => {
            let bits_43 = (opcode & 0b0001_1000) >> 3;
            match bits_43 {
                // jr n8
                0b11 => {
                    i.op = JR::N8.into();
                    i.length = 2;
                    i.cycles = (12, 0);
                    return Ok(i);
                }
                // jr cond, n8
                _ => {
                    let c: Cond = bits_43.try_into()?;
                    i.op = JR::Cond(c).into();
                    i.length = 2;
                    i.cycles = (8, 12);
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
            i.cycles = (4, 0);
            i.z = FlagBehavior::Reset;
            i.n = FlagBehavior::Reset;
            i.h = FlagBehavior::Reset;
            i.c = FlagBehavior::Dependent;
            return Ok(i);
        }
        // rrca
        0b0000_1111 => {
            i.op = Operation::RRCA;
            i.length = 1;
            i.cycles = (4, 0);
            i.z = FlagBehavior::Reset;
            i.n = FlagBehavior::Reset;
            i.h = FlagBehavior::Reset;
            i.c = FlagBehavior::Dependent;
            return Ok(i);
        }
        // rla
        0b0001_0111 => {
            i.op = Operation::RLA;
            i.length = 1;
            i.cycles = (4, 0);
            i.z = FlagBehavior::Reset;
            i.n = FlagBehavior::Reset;
            i.h = FlagBehavior::Reset;
            i.c = FlagBehavior::Dependent;
            return Ok(i);
        }
        // rra
        0b0001_1111 => {
            i.op = Operation::RRA;
            i.length = 1;
            i.cycles = (4, 0);
            i.z = FlagBehavior::Reset;
            i.n = FlagBehavior::Reset;
            i.h = FlagBehavior::Reset;
            i.c = FlagBehavior::Dependent;
            return Ok(i);
        }
        // daa
        0b0010_0111 => {
            i.op = Operation::DAA;
            i.length = 1;
            i.cycles = (4, 0);
            i.h = FlagBehavior::Reset;
            i.c = FlagBehavior::Dependent;
            i.z = FlagBehavior::IfZero(FlagState::Set);
            return Ok(i);
        }
        // cpl
        0b0010_1111 => {
            i.op = Operation::CPL;
            i.length = 1;
            i.cycles = (4, 0);
            i.n = FlagBehavior::Set;
            i.h = FlagBehavior::Set;
            return Ok(i);
        }
        // scf
        0b0011_0111 => {
            i.op = Operation::SCF;
            i.length = 1;
            i.cycles = (4, 0);
            i.n = FlagBehavior::Reset;
            i.h = FlagBehavior::Reset;
            i.c = FlagBehavior::Set;
            return Ok(i);
        }
        // ccf
        0b0011_1111 => {
            i.op = Operation::CCF;
            i.length = 1;
            i.cycles = (4, 0);
            i.n = FlagBehavior::Reset;
            i.h = FlagBehavior::Reset;
            i.c = FlagBehavior::Invert;
            return Ok(i);
        }
        _ => (),
    }

    Err(DecodeError::Unimplemented(opcode).into())
}
//
//fn decode_block_1(opcode: u8) -> Result<Instruction, InstructionError> {
//    if opcode == 0b0111_0110 {
//        return Ok(Instruction::halt());
//    } else {
//        // ld r8, r8
//        let dest = (opcode & 0b0011_1000) >> 4;
//        let dest = R8::from_u8(dest)?;
//        let src = (opcode & 0b0000_0111) >> 4;
//        let src = R8::from_u8(src)?;
//
//        let mut i = Instruction::new();
//        i.op = Operation::LD;
//        i.dest = Some(dest.into());
//        i.src = Some(src.into());
//
//        i.length = 1;
//        if dest == R8::HlMem || src == R8::HlMem {
//            i.cycles = 8;
//        } else {
//            i.cycles = 4;
//        }
//
//        return Ok(i);
//    }
//}
//
//fn decode_block_2(opcode: u8) -> Result<Instruction, InstructionError> {
//    let mut i = Instruction::new();
//    let operand = R8::from_u8(opcode & 0b0000_0111)?;
//    i.dest = Some(R8::A.into());
//    i.src = Some(operand.into());
//
//    i.length = 1;
//    if operand == R8::HlMem {
//        i.cycles = 8;
//    } else {
//        i.cycles = 4;
//    }
//
//    match (opcode & 0b1111_1000) >> 3 {
//        // add a, r8
//        0b1_0000 => {
//            i.op = Add::R8(R8::A, operand).into();
//            i.ex = |i, cpu| {
//                let a = cpu.read_byte(i.dest())?;
//                let val = cpu.read_byte(i.src())?;
//                let result = a + val;
//                cpu.write_byte(i.dest(), result)?;
//                if result == 0 {
//                    cpu.flag_set(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                if bit::add_overflow(a, val, 3) {
//                    cpu.flag_set(Flag::HC);
//                }
//                if bit::add_overflow(a, val, 7) {
//                    cpu.flag_set(Flag::C);
//                }
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // adc a, r8
//        0b1_0001 => {
//            i.op = Operation::ADC;
//            i.ex = |i, cpu| {
//                let a = cpu.read_byte(i.dest())?;
//                let cf = cpu.flag(Flag::C);
//                let val = cpu.read_byte(i.src())?;
//                let result = a + val + cf;
//                cpu.write_byte(i.dest(), result)?;
//                if result == 0 {
//                    cpu.flag_set(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                if bit::add_overflow(a, val + cf, 3) {
//                    cpu.flag_set(Flag::HC);
//                }
//                if bit::add_overflow(a, val + cf, 7) {
//                    cpu.flag_set(Flag::C);
//                }
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // sub a, r8
//        0b1_0010 => {
//            i.op = Operation::SUB;
//            i.ex = |i, cpu| {
//                let a = cpu.read_byte(i.dest())?;
//                let val = cpu.read_byte(i.src())?;
//                let result = a - val;
//                cpu.write_byte(i.dest(), result)?;
//                if result == 0 {
//                    cpu.flag_set(Flag::Z);
//                }
//                cpu.flag_set(Flag::N);
//                if bit::sub_borrow(a, val, 4) {
//                    cpu.flag_set(Flag::HC);
//                }
//                if bit::sub_borrow(a, val, 8) {
//                    cpu.flag_set(Flag::C);
//                }
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // sbc a, r8
//        0b1_0011 => {
//            i.op = Operation::SBC;
//            i.ex = |i, cpu| {
//                let a = cpu.read_byte(i.dest())?;
//                let cf = cpu.flag(Flag::C);
//                let val = cpu.read_byte(i.src())?;
//                let result = a - (val + cf);
//                cpu.write_byte(i.dest(), result)?;
//                if result == 0 {
//                    cpu.flag_set(Flag::Z);
//                }
//                cpu.flag_set(Flag::N);
//                if bit::sub_borrow(a, val, 4) {
//                    cpu.flag_set(Flag::HC);
//                }
//                if bit::sub_borrow(a, val + cf, 8) {
//                    cpu.flag_set(Flag::C);
//                }
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // and a, r8
//        0b1_0100 => {
//            i.op = Operation::AND;
//            i.ex = |i, cpu| {
//                let a = cpu.read_byte(i.dest())?;
//                let val = cpu.read_byte(i.src())?;
//                let result = a & val;
//                cpu.write_byte(i.dest(), result)?;
//                if result == 0 {
//                    cpu.flag_set(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                cpu.flag_set(Flag::HC);
//                cpu.flag_reset(Flag::C);
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // xor a, r8
//        0b1_0101 => {
//            i.op = Operation::XOR;
//            i.ex = |i, cpu| {
//                let a = cpu.read_byte(i.dest())?;
//                let val = cpu.read_byte(i.src())?;
//                let result = a ^ val;
//                cpu.write_byte(i.dest(), result)?;
//                if result == 0 {
//                    cpu.flag_set(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                cpu.flag_reset(Flag::HC);
//                cpu.flag_reset(Flag::C);
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // or a, r8
//        0b1_0110 => {
//            i.op = Operation::OR;
//            i.ex = |i, cpu| {
//                let a = cpu.read_byte(i.dest())?;
//                let val = cpu.read_byte(i.src())?;
//                let result = a | val;
//                cpu.write_byte(i.dest(), result)?;
//                if result == 0 {
//                    cpu.flag_set(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                cpu.flag_reset(Flag::HC);
//                cpu.flag_reset(Flag::C);
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // cp a, r8
//        0b1_0111 => {
//            i.op = Operation::CP;
//            i.ex = |i, cpu| {
//                let a = cpu.read_byte(i.dest())?;
//                let val = cpu.read_byte(i.src())?;
//                let result = a - val;
//                if result == 0 {
//                    cpu.flag_set(Flag::Z);
//                }
//                cpu.flag_set(Flag::N);
//                if bit::sub_borrow(a, val, 4) {
//                    cpu.flag_set(Flag::HC);
//                }
//                if bit::sub_borrow(a, val, 8) {
//                    cpu.flag_set(Flag::C);
//                }
//                Ok(())
//            };
//            return Ok(i);
//        }
//        _ => (),
//    }
//
//    Err(InstructionError::Unimplemented(opcode))
//}
//
//const INVALID_OPCODES: [u8; 11] = [
//    0xD3, 0xDB, 0xDD, 0xE3, 0xE4, 0xEB, 0xEC, 0xED, 0xF4, 0xFC, 0xFD,
//];
//
//fn decode_block_3(opcode: u8) -> Result<Instruction, InstructionError> {
//    if INVALID_OPCODES.contains(&opcode) {
//        return Err(InstructionError::InvalidOpCode(opcode));
//    }
//
//    let mut i = Instruction::new();
//    // Prefix
//    if opcode == 0b1100_1011 {
//        i.op = Operation::PREFIX;
//        i.length = 1;
//        i.cycles = 4;
//        return Ok(i);
//    }
//
//    if (opcode & 0b111) == 0b110 {
//        i.dest = Some(R8::A.into());
//        i.src = Some(Operand::Imm8);
//        i.length = 2;
//        i.cycles = 8;
//
//        match (opcode & 0b0011_1000) >> 3 {
//            // add a, imm8
//            0 => {
//                i.op = Operation::ADD;
//                i.ex = |i, cpu| {
//                    let a = cpu.read_byte(i.dest())?;
//                    let val = cpu.fetch()?;
//                    let result = a + val;
//                    cpu.write_byte(i.dest(), result)?;
//                    if result == 0 {
//                        cpu.flag_set(Flag::Z);
//                    }
//                    cpu.flag_reset(Flag::N);
//                    if bit::add_overflow(a, val, 3) {
//                        cpu.flag_set(Flag::HC);
//                    }
//                    if bit::add_overflow(a, val, 7) {
//                        cpu.flag_set(Flag::C);
//                    }
//                    Ok(())
//                };
//                return Ok(i);
//            }
//            // adc a, imm8
//            0b1 => {
//                i.op = Operation::ADC;
//                i.ex = |i, cpu| {
//                    let a = cpu.read_byte(i.dest())?;
//                    let cf = cpu.flag(Flag::C);
//                    let val = cpu.fetch()?;
//                    let result = a + val + cf;
//                    cpu.write_byte(i.dest(), result)?;
//                    if result == 0 {
//                        cpu.flag_set(Flag::Z);
//                    }
//                    cpu.flag_reset(Flag::N);
//                    if bit::add_overflow(a, val + cf, 3) {
//                        cpu.flag_set(Flag::HC);
//                    }
//                    if bit::add_overflow(a, val + cf, 7) {
//                        cpu.flag_set(Flag::C);
//                    }
//                    Ok(())
//                };
//                return Ok(i);
//            }
//            // sub a, imm8
//            0b10 => {
//                i.op = Operation::SUB;
//                i.ex = |i, cpu| {
//                    let a = cpu.read_byte(i.dest())?;
//                    let val = cpu.fetch()?;
//                    let result = a - val;
//                    cpu.write_byte(i.dest(), result)?;
//                    if result == 0 {
//                        cpu.flag_set(Flag::Z);
//                    }
//                    cpu.flag_set(Flag::N);
//                    if bit::sub_borrow(a, val, 4) {
//                        cpu.flag_set(Flag::HC);
//                    }
//                    if bit::sub_borrow(a, val, 8) {
//                        cpu.flag_set(Flag::C);
//                    }
//                    Ok(())
//                };
//                return Ok(i);
//            }
//            // sbc a, imm8
//            0b11 => {
//                i.op = Operation::SBC;
//                i.ex = |i, cpu| {
//                    let a = cpu.read_byte(i.dest())?;
//                    let cf = cpu.flag(Flag::C);
//                    let val = cpu.fetch()?;
//                    let result = a - (val + cf);
//                    cpu.write_byte(i.dest(), result)?;
//                    if result == 0 {
//                        cpu.flag_set(Flag::Z);
//                    }
//                    cpu.flag_set(Flag::N);
//                    if bit::sub_borrow(a, val, 4) {
//                        cpu.flag_set(Flag::HC);
//                    }
//                    if bit::sub_borrow(a, val + cf, 8) {
//                        cpu.flag_set(Flag::C);
//                    }
//                    Ok(())
//                };
//                return Ok(i);
//            }
//            // and a, imm8
//            0b100 => {
//                i.op = Operation::AND;
//                i.ex = |i, cpu| {
//                    let a = cpu.read_byte(i.dest())?;
//                    let val = cpu.fetch()?;
//                    let result = a & val;
//                    cpu.write_byte(i.dest(), result)?;
//                    if result == 0 {
//                        cpu.flag_set(Flag::Z);
//                    }
//                    cpu.flag_reset(Flag::N);
//                    cpu.flag_set(Flag::HC);
//                    cpu.flag_reset(Flag::C);
//                    Ok(())
//                };
//                return Ok(i);
//            }
//            // xor a, imm8
//            0b101 => {
//                i.op = Operation::XOR;
//                i.ex = |i, cpu| {
//                    let a = cpu.read_byte(i.dest())?;
//                    let val = cpu.fetch()?;
//                    let result = a ^ val;
//                    cpu.write_byte(i.dest(), result)?;
//                    if result == 0 {
//                        cpu.flag_set(Flag::Z);
//                    }
//                    cpu.flag_reset(Flag::N);
//                    cpu.flag_reset(Flag::HC);
//                    cpu.flag_reset(Flag::C);
//                    Ok(())
//                };
//                return Ok(i);
//            }
//            // or a, imm8
//            0b110 => {
//                i.op = Operation::OR;
//                i.ex = |i, cpu| {
//                    let a = cpu.read_byte(i.dest())?;
//                    let val = cpu.fetch()?;
//                    let result = a | val;
//                    cpu.write_byte(i.dest(), result)?;
//                    if result == 0 {
//                        cpu.flag_set(Flag::Z);
//                    }
//                    cpu.flag_reset(Flag::N);
//                    cpu.flag_reset(Flag::HC);
//                    cpu.flag_reset(Flag::C);
//                    Ok(())
//                };
//                return Ok(i);
//            }
//            // cp a, imm8
//            0b111 => {
//                i.op = Operation::CP;
//                i.ex = |i, cpu| {
//                    let a = cpu.read_byte(i.dest())?;
//                    let val = cpu.fetch()?;
//                    let result = a - val;
//                    if result == 0 {
//                        cpu.flag_set(Flag::Z);
//                    }
//                    cpu.flag_set(Flag::N);
//                    if bit::sub_borrow(a, val, 4) {
//                        cpu.flag_set(Flag::HC);
//                    }
//                    if bit::sub_borrow(a, val, 8) {
//                        cpu.flag_set(Flag::C);
//                    }
//                    Ok(())
//                };
//                return Ok(i);
//            }
//            _ => (),
//        }
//
//        match opcode {
//            // ldh [c], a
//            0b1110_0010 => {
//                i.op = Operation::LDH;
//                i.dest = Some(Operand::C.to_mem());
//                i.src = Some(Operand::A);
//                i.length = 1;
//                i.cycles = 8;
//                i.ex = |i, cpu| {
//                    let c = cpu.read_byte(i.dest())? as u16;
//                    let addr = c + 0xFF00;
//                    let a = cpu.read_byte(i.src())?;
//                    let mu = cpu.mem();
//                    let mut mem = mu.lock().unwrap();
//                    mem.write(addr, a)?;
//                    Ok(())
//                };
//                return Ok(i);
//            }
//
//            // ldh [imm8], a
//            0b1110_0000 => {
//                i.op = Operation::LD;
//                i.dest = Some(Operand::Imm8.to_mem());
//                i.src = Some(Operand::A);
//                i.length = 2;
//                i.cycles = 12;
//                i.ex = |i, cpu| {
//                    let imm8 = cpu.fetch()? as u16;
//                    let addr = imm8 + 0xFF00;
//                    let a = cpu.read_byte(i.src())?;
//                    let mu = cpu.mem();
//                    let mut mem = mu.lock().unwrap();
//                    mem.write(addr, a)?;
//                    Ok(())
//                };
//                return Ok(i);
//            }
//
//            // ld [imm16], a
//            0b1110_1010 => {
//                i.op = Operation::LD;
//                i.dest = Some(Operand::Imm16.to_mem());
//                i.src = Some(Operand::A);
//                i.length = 3;
//                i.cycles = 16;
//                i.ex = |i, cpu| {
//                    let imm16 = cpu.fetch_word()?;
//                    let a = cpu.read_byte(i.src())?;
//                    let mu = cpu.mem();
//                    let mut mem = mu.lock().unwrap();
//                    mem.write(imm16, a)?;
//                    Ok(())
//                };
//                return Ok(i);
//            }
//
//            // ldh a, [c]
//            0b1111_0010 => {
//                i.op = Operation::LDH;
//                i.dest = Some(Operand::A);
//                i.src = Some(Operand::C.to_mem());
//                i.length = 1;
//                i.cycles = 8;
//                i.ex = |i, cpu| {
//                    let c = cpu.read_byte(i.dest())? as u16;
//                    let addr = c + 0xFF00;
//                    let mu = cpu.mem();
//                    let mem = mu.lock().unwrap();
//                    let val = mem.read(addr)?;
//                    cpu.write_byte(i.dest(), val)?;
//                    Ok(())
//                };
//                return Ok(i);
//            }
//
//            // ldh a, [imm8]
//            0b1111_0000 => {
//                i.op = Operation::LDH;
//                i.dest = Some(Operand::A);
//                i.src = Some(Operand::Imm8.to_mem());
//                i.length = 2;
//                i.cycles = 12;
//                i.ex = |i, cpu| {
//                    let imm8 = cpu.fetch()? as u16;
//                    let addr = imm8 + 0xFF00;
//                    let mu = cpu.mem();
//                    let mem = mu.lock().unwrap();
//                    let val = mem.read(addr)?;
//                    cpu.write_byte(i.dest(), val)?;
//                    Ok(())
//                };
//                return Ok(i);
//            }
//
//            // ld a, [imm16]
//            0b1111_1010 => {
//                i.op = Operation::LD;
//                i.dest = Some(Operand::A);
//                i.src = Some(Operand::Imm16.to_mem());
//                i.length = 3;
//                i.cycles = 16;
//                i.ex = |i, cpu| {
//                    let imm16 = cpu.fetch_word()?;
//                    let mu = cpu.mem();
//                    let mem = mu.lock().unwrap();
//                    let val = mem.read(imm16)?;
//                    cpu.write_byte(i.dest(), val)?;
//                    Ok(())
//                };
//                return Ok(i);
//            }
//
//            // add sp, imm8
//            0b1110_1000 => {
//                i.op = Operation::ADD;
//                i.dest = Some(R16::SP.to_operand());
//                i.src = Some(Operand::Imm8);
//                i.length = 2;
//                i.cycles = 16;
//                i.ex = |i, cpu| {
//                    let sp = cpu.read(i.dest())?;
//                    let imm8 = cpu.fetch()? as u16;
//                    let result = sp + imm8;
//                    cpu.write(i.dest(), result)?;
//                    cpu.flag_reset(Flag::Z);
//                    cpu.flag_reset(Flag::N);
//                    if bit::add_overflow_u16(sp, imm8, 3) {
//                        cpu.flag_set(Flag::HC);
//                    }
//                    if bit::add_overflow_u16(sp, imm8, 7) {
//                        cpu.flag_set(Flag::C);
//                    }
//                    Ok(())
//                };
//                return Ok(i);
//            }
//
//            // ld hl, sp + imm8
//            0b1111_1000 => {
//                i.op = Operation::LD;
//                i.dest = Some(R16::HL.to_operand());
//                let sp = Box::new(R16::SP.to_operand());
//                let imm8 = Box::new(Operand::Imm8);
//                i.src = Some(Operand::Sum((sp, imm8)));
//                i.length = 2;
//                i.cycles = 12;
//                i.ex = |i, cpu| {
//                    let sp = cpu.read(i.dest())?;
//                    let imm8 = cpu.fetch()? as u16;
//                    let result = sp + imm8;
//                    cpu.write(Some(Operand::HL), result)?;
//
//                    cpu.flag_reset(Flag::Z);
//                    cpu.flag_reset(Flag::N);
//                    if bit::add_overflow_u16(sp, imm8, 3) {
//                        cpu.flag_set(Flag::HC);
//                    }
//                    if bit::add_overflow_u16(sp, imm8, 7) {
//                        cpu.flag_set(Flag::C);
//                    }
//                    Ok(())
//                };
//                return Ok(i);
//            }
//
//            // ld sp, hl
//            0b1111_1001 => {
//                i.op = Operation::LD;
//                i.dest = Some(R16::SP.to_operand());
//                i.src = Some(R16::HL.to_operand());
//                i.length = 1;
//                i.cycles = 8;
//                i.ex = |i, cpu| {
//                    let hl = cpu.read(i.src())?;
//                    cpu.write(i.dest(), hl)?;
//                    Ok(())
//                };
//                return Ok(i);
//            }
//
//            // di
//            0b1111_0011 => {
//                i.op = Operation::DI;
//                i.length = 1;
//                i.cycles = 4;
//                i.ex = |_, cpu| {
//                    cpu.disable_interrupts();
//                    Ok(())
//                };
//                return Ok(i);
//            }
//
//            // ei
//            0b1111_1011 => {
//                i.op = Operation::EI;
//                i.length = 1;
//                i.cycles = 4;
//                i.ex = |_, cpu| {
//                    cpu.enable_interrupts();
//                    Ok(())
//                };
//                return Ok(i);
//            }
//
//            // ret
//            0b1100_1001 => {
//                i.op = Operation::RET;
//                i.length = 1;
//                i.cycles = 16;
//                i.ex = |_, cpu| {
//                    let sp = cpu.pop_stack()?;
//                    cpu.set_pc(sp);
//                    Ok(())
//                };
//                return Ok(i);
//            }
//            // reti
//            0b1101_1001 => {
//                i.op = Operation::RET;
//                i.length = 1;
//                i.cycles = 16;
//                i.ex = |_, cpu| {
//                    let sp_val = cpu.pop_stack()?;
//                    cpu.set_pc(sp_val);
//                    cpu.enable_interrupts();
//                    Ok(())
//                };
//            }
//
//            // jp imm16
//            0b1100_0011 => {
//                i.op = Operation::JP;
//                i.dest = Some(Operand::Imm16);
//                i.length = 3;
//                i.cycles = 16;
//                i.ex = |_, cpu| {
//                    let imm16 = cpu.fetch_word()?;
//                    cpu.set_pc(imm16);
//                    Ok(())
//                };
//                return Ok(i);
//            }
//            // jp hl
//            0b1110_1001 => {
//                i.op = Operation::JP;
//                i.dest = Some(Operand::HL);
//                i.length = 3;
//                i.cycles = 16;
//                i.ex = |i, cpu| {
//                    let hl = cpu.read(i.dest())?;
//                    cpu.set_pc(hl);
//                    Ok(())
//                };
//                return Ok(i);
//            }
//
//            // call imm16
//            0b1100_1101 => {
//                i.op = Operation::CALL;
//                i.dest = Some(Operand::Imm16);
//                i.length = 3;
//                i.cycles = 24;
//                i.ex = |_, cpu| {
//                    let imm16 = cpu.fetch_word()?;
//                    let pc = cpu.get_pc();
//                    cpu.push_stack(pc)?;
//                    cpu.set_pc(imm16);
//                    Ok(())
//                };
//                return Ok(i);
//            }
//            _ => (),
//        }
//    }
//
//    let last_three = opcode & 0b111;
//    match last_three {
//        // ret cond
//        0b000 => {
//            i.op = Operation::RET;
//            let c = Cond::from_u8((opcode & 0b0001_1000) >> 3)?;
//            i.dest = Some(c.to_operand());
//            i.length = 1;
//            i.cycles = 8;
//            i.branch_cycles = 20;
//            i.ex = |i, cpu| {
//                if cpu.cc(i.dest())? {
//                    let sp = cpu.read(Some(Operand::SP))?;
//                    cpu.set_pc(sp);
//                    let sp = sp + 2;
//                    cpu.write(Some(Operand::SP), sp)?;
//                }
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // jp cond, imm16
//        0b010 => {
//            i.op = Operation::JP;
//            let c = Cond::from_u8((opcode & 0b0001_1000) >> 3)?;
//            i.dest = Some(c.to_operand());
//            i.src = Some(Operand::Imm16);
//            i.length = 3;
//            i.cycles = 12;
//            i.branch_cycles = 16;
//            i.ex = |i, cpu| {
//                if cpu.cc(i.dest())? {
//                    let imm16 = cpu.fetch_word()?;
//                    cpu.set_pc(imm16);
//                }
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // call cond, imm16
//        0b100 => {
//            i.op = Operation::CALL;
//            let c = Cond::from_u8((opcode & 0b0001_1000) >> 3)?;
//            i.dest = Some(c.to_operand());
//            i.src = Some(Operand::Imm16);
//            i.length = 3;
//            i.cycles = 12;
//            i.branch_cycles = 24;
//            i.ex = |i, cpu| {
//                if cpu.cc(i.dest())? {
//                    let imm16 = cpu.fetch_word()?;
//                    let pc = cpu.get_pc();
//                    cpu.push_stack(pc)?;
//                    cpu.set_pc(imm16);
//                }
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // rst tgt3
//        0b111 => {
//            i.op = Operation::RST;
//            let t = Tgt3::new((opcode & 0b0011_1000) >> 3);
//            i.dest = Some(Operand::Tgt3(t));
//            i.length = 1;
//            i.cycles = 16;
//            i.ex = |i, cpu| {
//                let t: Tgt3 = i.dest().into();
//                let t = t.addr();
//                let pc = cpu.get_pc();
//                cpu.push_stack(pc)?;
//                cpu.set_pc(t as u16);
//                Ok(())
//            };
//            return Ok(i);
//        }
//        _ => (),
//    }
//
//    let last_four = opcode & 0b1111;
//    match last_four {
//        // pop r16stk
//        0b0001 => {
//            i.op = Operation::POP;
//            let r = R16Stk::from_u8((opcode & 0b0011_0000) >> 4)?;
//            i.dest = Some(r.to_operand());
//            i.length = 1;
//            i.cycles = 12;
//            i.ex = |i, cpu| {
//                let sp_val = cpu.pop_stack()?;
//                cpu.write(i.dest(), sp_val)?;
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // push r16stk
//        0b0101 => {
//            i.op = Operation::PUSH;
//            let r = R16Stk::from_u8((opcode & 0b0011_0000) >> 4)?;
//            i.dest = Some(r.to_operand());
//            i.length = 1;
//            i.cycles = 16;
//            i.ex = |i, cpu| {
//                let r16_val = cpu.read(i.dest())?;
//                cpu.push_stack(r16_val)?;
//                Ok(())
//            };
//            return Ok(i);
//        }
//        _ => (),
//    }
//
//    Err(InstructionError::Unknown)
//}
//
//fn decode_prefix(opcode: u8) -> Result<Instruction, InstructionError> {
//    let mut i = Instruction::new();
//
//    let first_five = (opcode & 0b1111_1000) >> 3;
//    match first_five {
//        // rlc r8
//        0b000 => {
//            i.op = Operation::RLC;
//            let r = R8::from_u8(opcode & 0b111)?;
//            i.dest = Some(r.into());
//            i.length = 2;
//            if r == R8::HlMem {
//                i.cycles = 16;
//            } else {
//                i.cycles = 8;
//            }
//            i.ex = |i, cpu| {
//                let r = cpu.read_byte(i.dest())?;
//                let r7 = bit::get(r, 7);
//                cpu.flag_set_val(Flag::C, r7);
//                let r = (r << 1) + r7;
//                if r == 0 {
//                    cpu.flag_reset(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                cpu.flag_reset(Flag::HC);
//                cpu.write_byte(i.dest(), r)?;
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // rrc r8
//        0b001 => {
//            i.op = Operation::RRC;
//            let r = R8::from_u8(opcode & 0b111)?;
//            i.dest = Some(r.into());
//            i.length = 2;
//            if r == R8::HlMem {
//                i.cycles = 16;
//            } else {
//                i.cycles = 8;
//            }
//            i.ex = |i, cpu| {
//                let r = cpu.read_byte(i.dest())?;
//                let r0 = bit::get(r, 0);
//                cpu.flag_set_val(Flag::C, r0);
//                let r = (r >> 1) + (r0 << 7);
//                if r == 0 {
//                    cpu.flag_reset(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                cpu.flag_reset(Flag::HC);
//                cpu.write_byte(i.dest(), r)?;
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // rl r8
//        0b010 => {
//            i.op = Operation::RL;
//            let r = R8::from_u8(opcode & 0b111)?;
//            i.dest = Some(r.into());
//            i.length = 2;
//            if r == R8::HlMem {
//                i.cycles = 16;
//            } else {
//                i.cycles = 8;
//            }
//            i.ex = |i, cpu| {
//                let r = cpu.read_byte(i.dest())?;
//                let cf = cpu.flag(Flag::C);
//                let r7 = bit::get(r, 7);
//                cpu.flag_set_val(Flag::C, r7);
//                let r = (r << 1) + cf;
//                if r == 0 {
//                    cpu.flag_reset(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                cpu.flag_reset(Flag::HC);
//                cpu.write_byte(i.dest(), r)?;
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // rr r8
//        0b011 => {
//            i.op = Operation::RR;
//            let r = R8::from_u8(opcode & 0b111)?;
//            i.dest = Some(r.into());
//            i.length = 2;
//            if r == R8::HlMem {
//                i.cycles = 16;
//            } else {
//                i.cycles = 8;
//            }
//            i.ex = |i, cpu| {
//                let r = cpu.read_byte(i.dest())?;
//                let cf = cpu.flag(Flag::C);
//                let r0 = bit::get(r, 0);
//                cpu.flag_set_val(Flag::C, r0);
//                let r = (r >> 1) + (cf << 7);
//                if r == 0 {
//                    cpu.flag_reset(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                cpu.flag_reset(Flag::HC);
//                cpu.write_byte(i.dest(), r)?;
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // sla r8
//        0b100 => {
//            i.op = Operation::SLA;
//            let r = R8::from_u8(opcode & 0b111)?;
//            i.dest = Some(r.into());
//            i.length = 2;
//            if r == R8::HlMem {
//                i.cycles = 16;
//            } else {
//                i.cycles = 8;
//            }
//            i.ex = |i, cpu| {
//                let r = cpu.read_byte(i.dest())?;
//                let r7 = bit::get(r, 7);
//                cpu.flag_set_val(Flag::C, r7);
//                let r = r << 1;
//                if r == 0 {
//                    cpu.flag_reset(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                cpu.flag_reset(Flag::HC);
//                cpu.write_byte(i.dest(), r)?;
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // sra r8
//        0b101 => {
//            i.op = Operation::SRA;
//            let r = R8::from_u8(opcode & 0b111)?;
//            i.dest = Some(r.into());
//            i.length = 2;
//            if r == R8::HlMem {
//                i.cycles = 16;
//            } else {
//                i.cycles = 8;
//            }
//            i.ex = |i, cpu| {
//                let r = cpu.read_byte(i.dest())?;
//                let r0 = bit::get(r, 0);
//                let r7 = bit::get(r, 7) << 7;
//                cpu.flag_set_val(Flag::C, r0);
//                let r = r >> 1 | r7;
//                if r == 0 {
//                    cpu.flag_reset(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                cpu.flag_reset(Flag::HC);
//                cpu.write_byte(i.dest(), r)?;
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // swap r8
//        0b110 => {
//            i.op = Operation::RL;
//            let r = R8::from_u8(opcode & 0b111)?;
//            i.dest = Some(r.into());
//            i.length = 2;
//            if r == R8::HlMem {
//                i.cycles = 16;
//            } else {
//                i.cycles = 8;
//            }
//            i.ex = |i, cpu| {
//                let r = cpu.read_byte(i.dest())?;
//                let l = (r & 0b1111) << 4;
//                let h = (r & 0b1111_0000) >> 4;
//                let r = l | h;
//                cpu.write_byte(i.dest(), r)?;
//                if r == 0 {
//                    cpu.flag_reset(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                cpu.flag_reset(Flag::HC);
//                cpu.flag_reset(Flag::C);
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // srl r8
//        0b111 => {
//            i.op = Operation::SRL;
//            let r = R8::from_u8(opcode & 0b111)?;
//            i.dest = Some(r.into());
//            i.length = 2;
//            if r == R8::HlMem {
//                i.cycles = 16;
//            } else {
//                i.cycles = 8;
//            }
//            i.ex = |i, cpu| {
//                let r = cpu.read_byte(i.dest())?;
//                let r0 = bit::get(r, 0);
//                cpu.flag_set_val(Flag::C, r0);
//                let r = r >> 1;
//                if r == 0 {
//                    cpu.flag_reset(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                cpu.flag_reset(Flag::HC);
//                cpu.write_byte(i.dest(), r)?;
//                Ok(())
//            };
//            return Ok(i);
//        }
//        _ => (),
//    }
//
//    let last_two = (opcode & 0b1100_0000) >> 6;
//    match last_two {
//        // bit b3, r8
//        0b01 => {
//            i.op = Operation::BIT;
//            let b3 = BitIndex::new((opcode & 0b11_1000) >> 3);
//            let r = R8::from_u8(opcode & 0b111)?;
//            i.dest = Some(b3.to_operand());
//            i.src = Some(r.into());
//            i.length = 2;
//            if r == R8::HlMem {
//                i.cycles = 16;
//            } else {
//                i.cycles = 8;
//            }
//            i.ex = |i, cpu| {
//                let r = cpu.read_byte(i.src())?;
//                let b3: BitIndex = i.dest().into();
//                let b3 = b3.index();
//                if bit::is_set(r, b3) {
//                    cpu.flag_set(Flag::Z);
//                }
//                cpu.flag_reset(Flag::N);
//                cpu.flag_set(Flag::HC);
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // res b3, r8
//        0b10 => {
//            i.op = Operation::BIT;
//            let b3 = BitIndex::new((opcode & 0b11_1000) >> 3);
//            let r = R8::from_u8(opcode & 0b111)?;
//            i.dest = Some(b3.to_operand());
//            i.src = Some(b3.to_operand());
//            i.length = 2;
//            if r == R8::HlMem {
//                i.cycles = 16;
//            } else {
//                i.cycles = 8;
//            }
//            i.ex = |i, cpu| {
//                let mut r = cpu.read_byte(i.src())?;
//                let b3: BitIndex = i.dest().into();
//                let b3 = b3.index();
//                bit::reset(&mut r, b3);
//                cpu.write_byte(i.src(), r)?;
//                Ok(())
//            };
//            return Ok(i);
//        }
//        // set b3, r8
//        0b11 => {
//            i.op = Operation::BIT;
//            let b3 = BitIndex::new((opcode & 0b11_1000) >> 3);
//            let r = R8::from_u8(opcode & 0b111)?;
//            i.dest = Some(b3.to_operand());
//            i.src = Some(r.into());
//            i.length = 2;
//            if r == R8::HlMem {
//                i.cycles = 16;
//            } else {
//                i.cycles = 8;
//            }
//            i.ex = |i, cpu| {
//                let mut r = cpu.read_byte(i.src())?;
//                let b3: BitIndex = i.dest().into();
//                let b3 = b3.index();
//                bit::set(&mut r, b3);
//                cpu.write_byte(i.src(), r)?;
//                Ok(())
//            };
//            return Ok(i);
//        }
//        _ => (),
//    }
//
//    Err(InstructionError::Unknown)
//}
