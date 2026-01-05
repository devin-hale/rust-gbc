use thiserror::Error;

pub enum Block {
    Block0,
    Block1,
    Block2,
    Block3,
    Prefix,
}

#[derive(Debug, PartialEq)]
pub enum Operand {
    Direct(Src),
    Mem(Src),
}

#[derive(Debug, PartialEq)]
pub enum Src {
    R8(R8),
    R16(R16),
    R16Stk(R16Stk),
    R16Mem(R16Mem),
    Cond(Cond),
    B3(BitIndex),
    Tgt3(Tgt3),
    Imm8,
    Imm16,
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

#[derive(Debug, PartialEq)]
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
}

#[derive(Debug, PartialEq)]
pub enum R16Stk {
    BC,
    DE,
    HL,
    AF,
}

#[derive(Debug, PartialEq)]
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
}

#[derive(Debug, PartialEq)]
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
}

#[derive(Debug, PartialEq)]
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
}

#[derive(Debug, PartialEq)]
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlagBehavior {
    Unmodified,
    Set,
    Reset,
    Dependent,
}

pub enum Flag {
    Zero,
    Negative,
    HalfCarry,
    Carry,
}

pub struct FlagSet {
    zero: FlagBehavior,
    negative: FlagBehavior,
    half_carry: FlagBehavior,
    carry: FlagBehavior,
}

impl FlagSet {
    fn new() -> FlagSet {
        FlagSet {
            zero: FlagBehavior::Unmodified,
            negative: FlagBehavior::Unmodified,
            half_carry: FlagBehavior::Unmodified,
            carry: FlagBehavior::Unmodified,
        }
    }

    pub fn zero(&self) -> FlagBehavior {
        self.zero
    }
    pub fn negative(&self) -> FlagBehavior {
        self.negative
    }
    pub fn half_carry(&self) -> FlagBehavior {
        self.half_carry
    }
    pub fn carry(&self) -> FlagBehavior {
        self.carry
    }
    pub fn flag(&self, f: Flag) -> FlagBehavior {
        match f {
            Flag::Zero => self.zero,
            Flag::Negative => self.negative,
            Flag::HalfCarry => self.half_carry,
            Flag::Carry => self.carry,
        }
    }
    pub fn set(&mut self, f: Flag, fb: FlagBehavior) {
        match f {
            Flag::Zero => self.zero = fb,
            Flag::Negative => self.negative = fb,
            Flag::HalfCarry => self.half_carry = fb,
            Flag::Carry => self.carry = fb,
        }
    }
}

pub struct Instruction {
    op: Operation,
    operand_0: Option<Operand>,
    operand_1: Option<Operand>,

    length: u8,
    cycles: u8,
    branch_cycles: u8,

    flags: FlagSet,
}

impl Instruction {
    pub fn nop() -> Instruction {
        Instruction {
            op: Operation::NOP,
            operand_0: None,
            operand_1: None,
            length: 1,
            cycles: 4,
            branch_cycles: 0,
            flags: FlagSet::new(),
        }
    }
    pub fn stop() -> Instruction {
        let mut i = Instruction::nop();
        i.op = Operation::STOP;
        i
    }
    pub fn halt() -> Instruction {
        let mut i = Instruction::nop();
        i.op = Operation::HALT;
        i
    }
    pub fn new() -> Instruction {
        Instruction::nop()
    }
    pub fn flags(&self) -> &FlagSet {
        &self.flags
    }
    pub fn flags_mut(&mut self) -> &mut FlagSet {
        &mut self.flags
    }
}

#[derive(Debug, Error)]
pub enum InstructionError {
    #[error("invalid R16 value '{0}'")]
    InvalidR16(u8),

    #[error("invalid R16Mem value '{0}'")]
    InvalidR16Mem(u8),

    #[error("invalid R8 value '{0}")]
    InvalidR8(u8),

    #[error("invalid Cond value '{0}")]
    InvalidCond(u8),

    #[error("unimplemented instruction '{0:b}")]
    Unimplemented(u8),

    #[error("unknown instruction error")]
    Unknown,
}

pub fn decode(opcode: u8) -> Result<Instruction, InstructionError> {
    match (opcode >> 6) & 0x3 {
        0x00 => Ok(decode_block_0(opcode)?),
        0x01 => Ok(decode_block_1(opcode)?),
        //0x10 => (),
        //0x11 => (),
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
            i.operand_0 = Some(Operand::Direct(Src::R16(dest)));
            i.operand_1 = Some(Operand::Direct(Src::Imm16));
            i.length = 3;
            i.cycles = 12;
            return Ok(i);
        }
        // ld [r16mem], a
        0b0010 => {
            i.op = Operation::LD;
            let dest = R16Mem::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.operand_0 = Some(Operand::Mem(Src::R16Mem(dest)));
            i.operand_1 = Some(Operand::Direct(Src::R8(R8::A)));
            i.length = 1;
            i.cycles = 8;
            return Ok(i);
        }
        // ld a, [r16mem]
        0b1010 => {
            i.op = Operation::LD;
            let dest = R16Mem::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.operand_0 = Some(Operand::Direct(Src::R8(R8::A)));
            i.operand_1 = Some(Operand::Mem(Src::R16Mem(dest)));
            i.length = 1;
            i.cycles = 8;
            return Ok(i);
        }
        // ld [imm16], sp
        0b1000 => {
            let dest = (opcode & 0b0011_0000) >> 4;
            if dest == 0b00 {
                let dest = R16Mem::from_u8(dest)?;
                i.op = Operation::LD;
                i.operand_0 = Some(Operand::Mem(Src::Imm16));
                i.operand_1 = Some(Operand::Direct(Src::R16(R16::SP)));
                i.length = 3;
                i.cycles = 20;
                return Ok(i);
            }
        }

        // inc r16
        0b0011 => {
            i.op = Operation::INC;
            let r = R16::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.operand_0 = Some(Operand::Direct(Src::R16(r)));
            i.length = 1;
            i.cycles = 8;
            return Ok(i);
        }
        // dec r16
        0b1011 => {
            i.op = Operation::DEC;
            let r = R16::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.operand_0 = Some(Operand::Direct(Src::R16(r)));
            i.length = 1;
            i.cycles = 8;
            return Ok(i);
        }
        // add hl, r16
        0b1001 => {
            i.op = Operation::ADD;
            let r = R16::from_u8((opcode & 0b0011_0000) >> 4)?;
            i.operand_0 = Some(Operand::Direct(Src::R16(R16::HL)));
            i.operand_1 = Some(Operand::Direct(Src::R16(r)));
            i.length = 1;
            i.cycles = 8;
            i.flags.set(Flag::Negative, FlagBehavior::Reset);
            i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
            i.flags.set(Flag::Carry, FlagBehavior::Dependent);
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
            i.operand_0 = Some(Operand::Direct(Src::R8(r)));
            i.length = 1;
            i.cycles = 4;
            i.flags.set(Flag::Zero, FlagBehavior::Dependent);
            i.flags.set(Flag::Negative, FlagBehavior::Reset);
            i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
            return Ok(i);
        }

        // dec r8
        0b101 => {
            i.op = Operation::DEC;
            let r = R8::from_u8((opcode & 0b0011_1000) >> 3)?;
            i.operand_0 = Some(Operand::Direct(Src::R8(r)));
            i.length = 1;
            i.cycles = 4;
            i.flags.set(Flag::Zero, FlagBehavior::Dependent);
            i.flags.set(Flag::Negative, FlagBehavior::Set);
            i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
            return Ok(i);
        }

        // ld r8, imm8
        0b110 => {
            i.op = Operation::LD;
            let r = R8::from_u8((opcode & 0b0011_1000) >> 3)?;
            i.operand_0 = Some(Operand::Direct(Src::R8(r)));
            i.operand_1 = Some(Operand::Direct(Src::Imm8));
            i.length = 2;
            match r {
                R8::HlMem => i.cycles = 12,
                _ => i.cycles = 4,
            }
            return Ok(i);
        }

        0b000 => {
            let bits_43 = (opcode & 0b0001_1000) >> 3;
            match bits_43 {
                // jr imm8
                0b11 => {
                    i.op = Operation::JR;
                    i.operand_0 = Some(Operand::Direct(Src::Imm8));
                    i.length = 2;
                    i.cycles = 12;
                    return Ok(i);
                }
                // jr cond, imm8
                _ => {
                    i.op = Operation::JR;
                    let c = Cond::from_u8(bits_43)?;
                    i.operand_0 = Some(Operand::Direct(Src::Cond(c)));
                    i.operand_1 = Some(Operand::Direct(Src::Imm8));
                    i.length = 2;
                    i.cycles = 8;
                    i.branch_cycles = 12;
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
            return Ok(i);
        }
        // rrca
        0b0000_1111 => {
            i.op = Operation::RRCA;
            return Ok(i);
        }
        // rla
        0b0001_0111 => {
            i.op = Operation::RLA;
            return Ok(i);
        }
        // rra
        0b0001_1111 => {
            i.op = Operation::RRA;
            return Ok(i);
        }
        // daa
        0b0010_0111 => {
            i.op = Operation::DAA;
            return Ok(i);
        }
        // cpl
        0b0010_1111 => {
            i.op = Operation::CPL;
            return Ok(i);
        }
        // scf
        0b0011_0111 => {
            i.op = Operation::SCF;
            return Ok(i);
        }
        // ccf
        0b0011_1111 => {
            i.op = Operation::CCF;
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
        i.operand_0 = Some(Operand::Direct(Src::R8(dest)));
        i.operand_1 = Some(Operand::Direct(Src::R8(src)));

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
    i.operand_0 = Some(Operand::Direct(Src::R8(R8::A)));
    i.operand_1 = Some(Operand::Direct(Src::R8(operand)));

    if operand == R8::HlMem {
        i.cycles = 8;
    } else {
        i.cycles = 4;
    }
    match (opcode & 0b1111_1000) >> 3 {
        // add a, r8
        0b1_0000 => {
            i.op = Operation::ADD;
            i.flags.set(Flag::Zero, FlagBehavior::Dependent);
            i.flags.set(Flag::Negative, FlagBehavior::Reset);
            i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
            i.flags.set(Flag::Carry, FlagBehavior::Dependent);
            return Ok(i);
        }
        // adc a, r8
        0b1_0001 => {
            i.op = Operation::ADC;
            i.flags.set(Flag::Zero, FlagBehavior::Dependent);
            i.flags.set(Flag::Negative, FlagBehavior::Reset);
            i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
            i.flags.set(Flag::Carry, FlagBehavior::Dependent);
            return Ok(i);
        }
        // sub a, r8
        0b1_0010 => {
            i.op = Operation::SUB;
            i.flags.set(Flag::Zero, FlagBehavior::Dependent);
            i.flags.set(Flag::Negative, FlagBehavior::Set);
            i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
            i.flags.set(Flag::Carry, FlagBehavior::Dependent);
            return Ok(i);
        }
        // sbc a, r8
        0b1_0011 => {
            i.op = Operation::SBC;
            i.flags.set(Flag::Zero, FlagBehavior::Dependent);
            i.flags.set(Flag::Negative, FlagBehavior::Set);
            i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
            i.flags.set(Flag::Carry, FlagBehavior::Dependent);
            return Ok(i);
        }
        // and a, r8
        0b1_0100 => {
            i.op = Operation::AND;
            i.flags.set(Flag::Zero, FlagBehavior::Dependent);
            i.flags.set(Flag::Negative, FlagBehavior::Reset);
            i.flags.set(Flag::HalfCarry, FlagBehavior::Set);
            i.flags.set(Flag::Carry, FlagBehavior::Reset);
            return Ok(i);
        }
        // xor a, r8
        0b1_0101 => {
            i.op = Operation::XOR;
            i.op = Operation::AND;
            i.flags.set(Flag::Zero, FlagBehavior::Dependent);
            i.flags.set(Flag::Negative, FlagBehavior::Reset);
            i.flags.set(Flag::HalfCarry, FlagBehavior::Reset);
            i.flags.set(Flag::Carry, FlagBehavior::Reset);
            return Ok(i);
        }
        // or a, r8
        0b1_0110 => {
            i.op = Operation::OR;
            i.flags.set(Flag::Zero, FlagBehavior::Dependent);
            i.flags.set(Flag::Negative, FlagBehavior::Reset);
            i.flags.set(Flag::HalfCarry, FlagBehavior::Reset);
            i.flags.set(Flag::Carry, FlagBehavior::Reset);
            return Ok(i);
        }
        // cp a, r8
        0b1_0111 => {
            i.op = Operation::CP;
            i.flags.set(Flag::Zero, FlagBehavior::Dependent);
            i.flags.set(Flag::Negative, FlagBehavior::Set);
            i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
            i.flags.set(Flag::Carry, FlagBehavior::Dependent);
            return Ok(i);
        }
        _ => (),
    }

    Err(InstructionError::Unimplemented(opcode))
}

#[allow(unused_variables)]
fn decode_block_3(opcode: u8) -> Result<Instruction, InstructionError> {
    // Prefix
    if opcode == 0b1100_1011 {
        return Err(InstructionError::Unimplemented(opcode));
    }

    let mut i = Instruction::new();
    if (opcode & 0b111) == 0b110 {
        i.operand_0 = Some(Operand::Direct(Src::R8(R8::A)));
        i.operand_0 = Some(Operand::Direct(Src::Imm8));
        i.length = 2;
        i.cycles = 8;

        match (opcode & 0b0011_1000) >> 3 {
            // add a, imm8
            0 => {
                i.op = Operation::ADD;
                i.flags.set(Flag::Zero, FlagBehavior::Dependent);
                i.flags.set(Flag::Negative, FlagBehavior::Reset);
                i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
                i.flags.set(Flag::Carry, FlagBehavior::Dependent);
            }
            // adc a, imm8
            0b1 => {
                i.op = Operation::ADC;
                i.flags.set(Flag::Zero, FlagBehavior::Dependent);
                i.flags.set(Flag::Negative, FlagBehavior::Reset);
                i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
                i.flags.set(Flag::Carry, FlagBehavior::Dependent);
            }
            // sub a, imm8
            0b10 => {
                i.op = Operation::SUB;
                i.flags.set(Flag::Zero, FlagBehavior::Dependent);
                i.flags.set(Flag::Negative, FlagBehavior::Set);
                i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
                i.flags.set(Flag::Carry, FlagBehavior::Dependent);
            }
            // sbc a, imm8
            0b11 => {
                i.op = Operation::SBC;
                i.flags.set(Flag::Zero, FlagBehavior::Dependent);
                i.flags.set(Flag::Negative, FlagBehavior::Set);
                i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
                i.flags.set(Flag::Carry, FlagBehavior::Dependent);
            }
            // and a, imm8
            0b100 => {
                i.op = Operation::AND;
                i.flags.set(Flag::Zero, FlagBehavior::Dependent);
                i.flags.set(Flag::Negative, FlagBehavior::Reset);
                i.flags.set(Flag::HalfCarry, FlagBehavior::Set);
                i.flags.set(Flag::Carry, FlagBehavior::Reset);
            }
            // xor a, imm8
            0b101 => {
                i.op = Operation::XOR;
                i.flags.set(Flag::Zero, FlagBehavior::Dependent);
                i.flags.set(Flag::Negative, FlagBehavior::Reset);
                i.flags.set(Flag::HalfCarry, FlagBehavior::Reset);
                i.flags.set(Flag::Carry, FlagBehavior::Reset);
            }
            // or a, imm8
            0b110 => {
                i.op = Operation::OR;
                i.flags.set(Flag::Zero, FlagBehavior::Dependent);
                i.flags.set(Flag::Negative, FlagBehavior::Reset);
                i.flags.set(Flag::HalfCarry, FlagBehavior::Reset);
                i.flags.set(Flag::Carry, FlagBehavior::Reset);
            }
            // cp a, imm8
            0b111 => {
                i.op = Operation::CP;
                i.flags.set(Flag::Zero, FlagBehavior::Dependent);
                i.flags.set(Flag::Negative, FlagBehavior::Set);
                i.flags.set(Flag::HalfCarry, FlagBehavior::Dependent);
                i.flags.set(Flag::Carry, FlagBehavior::Dependent);
            }
            _ => (),
        }

        match opcode {
            // ld [c], a
            0b1110_0010 => {}
            // ld [imm8], a
            0b1110_0000 => {}
            // ld [imm16], a
            0b1110_1010 => {}
            // ldh a, [c]
            // ldh a, [imm8]
            // ld a, [imm16]
            // di
            0b1111_0011 => {}
            // ei
            0b1111_1011 => {}
            _ => ()
        }

        return Ok(i);
    }
    Err(InstructionError::Unknown)
}

#[cfg(test)]
mod test {
    use crate::instructions::{Cond, FlagBehavior, Operand, Operation, R8, R16, Src, decode};

    // BLOCK 0 TESTS
    #[test]
    fn decode_nop() {
        let i = decode(0).unwrap();
        assert_eq!(i.op, Operation::NOP);
        assert_eq!(i.operand_0, None);
        assert_eq!(i.operand_1, None);
        assert_eq!(i.length, 1);
        assert_eq!(i.cycles, 4);
    }

    fn decode_stop() {
        let i = decode(0x10).unwrap();
        assert_eq!(i.op, Operation::STOP);
        assert_eq!(i.operand_0, None);
        assert_eq!(i.operand_1, None);
        assert_eq!(i.length, 1);
        assert_eq!(i.cycles, 4);
    }

    #[test]
    fn decode_ld_r16_imm16() {
        let opcode = 0b0001_0001;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::LD);
        assert_eq!(i.operand_0, Some(Operand::Direct(Src::R16(R16::DE))));
        assert_eq!(i.operand_1, Some(Operand::Direct(Src::Imm16)));
        assert_eq!(i.length, 3);
        assert_eq!(i.cycles, 12);
    }

    #[test]
    fn decode_ld_r16mem_a() {
        let opcode = 0b0001_0010;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::LD);
        assert_eq!(
            i.operand_0,
            Some(Operand::Mem(Src::R16Mem(crate::instructions::R16Mem::DE)))
        );
        assert_eq!(i.operand_1, Some(Operand::Direct(Src::R8(R8::A))));
        assert_eq!(i.length, 1);
        assert_eq!(i.cycles, 8);
    }

    #[test]
    fn decode_ld_a_r16mem() {
        let opcode = 0b0001_1010;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::LD);
        assert_eq!(i.operand_0, Some(Operand::Direct(Src::R8(R8::A))));
        assert_eq!(
            i.operand_1,
            Some(Operand::Mem(Src::R16Mem(crate::instructions::R16Mem::DE)))
        );
        assert_eq!(i.length, 1);
        assert_eq!(i.cycles, 8);
    }

    #[test]
    fn decode_ld_imm16mem_sp() {
        let opcode = 0b0000_1000;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::LD);
        assert_eq!(i.operand_0, Some(Operand::Mem(Src::Imm16)));
        assert_eq!(i.operand_1, Some(Operand::Direct(Src::R16(R16::SP))));
        assert_eq!(i.length, 3);
        assert_eq!(i.cycles, 20);
    }

    #[test]
    fn decode_inc_r16() {
        let opcode = 0b0001_0011;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::INC);
        assert_eq!(i.operand_0, Some(Operand::Direct(Src::R16(R16::DE))));
        assert_eq!(i.operand_1, None);
        assert_eq!(i.length, 1);
        assert_eq!(i.cycles, 8);
    }

    #[test]
    fn decode_dec_r16() {
        let opcode = 0b0001_1011;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::DEC);
        assert_eq!(i.operand_0, Some(Operand::Direct(Src::R16(R16::DE))));
        assert_eq!(i.operand_1, None);
        assert_eq!(i.length, 1);
        assert_eq!(i.cycles, 8);
    }

    #[test]
    fn decode_add_hl_r16() {
        let opcode = 0x09;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::ADD);
        assert_eq!(i.operand_0, Some(Operand::Direct(Src::R16(R16::HL))));
        assert_eq!(i.operand_1, Some(Operand::Direct(Src::R16(R16::BC))));
        assert_eq!(i.flags.zero, FlagBehavior::Unmodified);
        assert_eq!(i.flags.negative, FlagBehavior::Reset);
        assert_eq!(i.flags.half_carry, FlagBehavior::Dependent);
        assert_eq!(i.flags.carry, FlagBehavior::Dependent);
    }

    #[test]
    fn decode_inc_r8() {
        let opcode = 0x04;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::INC);
        assert_eq!(i.operand_0, Some(Operand::Direct(Src::R8(R8::B))));
        assert_eq!(i.operand_1, None);
        assert_eq!(i.flags.zero, FlagBehavior::Dependent);
        assert_eq!(i.flags.negative, FlagBehavior::Reset);
        assert_eq!(i.flags.half_carry, FlagBehavior::Dependent);
        assert_eq!(i.flags.carry, FlagBehavior::Unmodified);
    }

    #[test]
    fn decode_dec_r8() {
        let opcode = 0x05;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::DEC);
        assert_eq!(i.operand_0, Some(Operand::Direct(Src::R8(R8::B))));
        assert_eq!(i.operand_1, None);
        assert_eq!(i.flags.zero, FlagBehavior::Dependent);
        assert_eq!(i.flags.negative, FlagBehavior::Set);
        assert_eq!(i.flags.half_carry, FlagBehavior::Dependent);
        assert_eq!(i.flags.carry, FlagBehavior::Unmodified);
    }

    #[test]
    fn decode_ld_r8_imm8() {
        let opcode = 0x36;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::LD);
        assert_eq!(i.operand_0, Some(Operand::Direct(Src::R8(R8::HlMem))));
        assert_eq!(i.operand_1, Some(Operand::Direct(Src::Imm8)));
    }

    #[test]
    fn decode_jr_imm8() {
        let opcode = 0x18;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::JR);
        assert_eq!(i.operand_0, Some(Operand::Direct(Src::Imm8)));
    }

    #[test]
    fn decode_jr_cond_imm8() {
        let opcode = 0x28;
        let i = decode(opcode).unwrap();
        assert_eq!(i.op, Operation::JR);
        assert_eq!(i.operand_0, Some(Operand::Direct(Src::Cond(Cond::Z))));
        assert_eq!(i.operand_1, Some(Operand::Direct(Src::Imm8)));
    }
}
