use std::sync::{Arc, Mutex};

use thiserror::Error;

use crate::{
    bit,
    instructions::{self, Instruction, InstructionError, Operand},
    memory::{Memory, MemoryError},
    registers::{Level, Register},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("CPU: operand value type mismatch")]
    OperandValueTypeMismiatch,
    #[error("CPU: memory error: {0}")]
    MemoryError(MemoryError),
    #[error("CPU: missing source operand")]
    MissingSource,
    #[error("CPU: missing destination operand")]
    MissingDestination,
    #[error("CPU: unknown error")]
    Unknown,
}

pub struct CPU {
    af: Register,
    bc: Register,
    de: Register,
    hl: Register,
    sp: Register,
    pc: Register,

    mem: Arc<Mutex<Memory>>,

    stopped: bool,
    halted: bool,
}

pub enum ByteRegister {
    B,
    C,
    D,
    E,
    H,
    L,
    A,
}

pub enum Flag {
    C = 4,
    HC,
    N,
    Z,
}

impl CPU {
    pub fn new(mem: Arc<Mutex<Memory>>) -> CPU {
        CPU {
            af: Register::new(),
            bc: Register::new(),
            de: Register::new(),
            hl: Register::new(),
            sp: Register::new(),
            pc: Register::new(),
            mem,
            stopped: false,
            halted: false,
        }
    }

    pub fn flags(&self) -> u8 {
        self.af().low()
    }

    pub fn byte(&self, r: Option<Operand>) -> Result<u8, &'static str> {
        match r {
            Some(o) => match o {
                Operand::B => Ok(self.bc.read_byte(Level::High)),
                Operand::D => Ok(self.de.read_byte(Level::High)),
                Operand::H => Ok(self.hl.read_byte(Level::High)),
                Operand::A => Ok(self.af.read_byte(Level::High)),
                Operand::C => Ok(self.bc.read_byte(Level::Low)),
                Operand::E => Ok(self.de.read_byte(Level::Low)),
                Operand::L => Ok(self.hl.read_byte(Level::Low)),
                Operand::Mem(o) => match *o {
                    Operand::HL => {
                        let addr = self.hl.val();
                        let mem = self.mem.lock().unwrap();
                        Ok(mem.read(addr).unwrap())
                    }
                    _ => return Err("invalid R8"),
                },
                _ => Err("invalid R8"),
            },
            None => Err("missing Operand"),
        }
    }

    pub fn flag_set(&mut self, f: Flag) {
        let mut flags = self.af.low();
        bit::set(&mut flags, f as u8);
        self.af.write_byte(Level::Low, flags);
    }

    pub fn flag_reset(&mut self, f: Flag) {
        let mut flags = self.af.low();
        bit::reset(&mut flags, f as u8);
        self.af.write_byte(Level::Low, flags);
    }

    pub fn flag_is_set(&self, f: Flag) -> bool {
        let flags = self.af.low();
        bit::get(flags, f as u8) == 1
    }

    pub fn write_byte(&mut self, r: Option<Operand>, data: u8) -> Result<(), &'static str> {
        match r {
            Some(o) => match o {
                Operand::B => self.bc.write_byte(Level::High, data),
                Operand::D => self.de.write_byte(Level::High, data),
                Operand::H => self.hl.write_byte(Level::High, data),
                Operand::A => self.af.write_byte(Level::High, data),
                Operand::C => self.bc.write_byte(Level::Low, data),
                Operand::E => self.de.write_byte(Level::Low, data),
                Operand::L => self.hl.write_byte(Level::Low, data),
                Operand::Mem(o) => match *o {
                    Operand::HL => {
                        let addr = self.hl.val();
                        let mut mem = self.mem.lock().unwrap();
                        mem.write(addr, data).unwrap();
                    }
                    _ => return Err("invalid R8"),
                },
                _ => {
                    return Err("invalid R8");
                }
            },
            None => {
                return Err("missing Operand");
            }
        }
        Ok(())
    }

    pub fn inc_byte(&mut self, r: Option<Operand>) -> Result<u8, &'static str> {
        let data = self.byte(r.clone())? + 1;
        self.write_byte(r.clone(), data)?;
        Ok(data)
    }

    pub fn dec_byte(&mut self, r: Option<Operand>) -> Result<u8, &'static str> {
        let data = self.byte(r.clone())? - 1;
        self.write_byte(r.clone(), data)?;
        Ok(data)
    }

    pub fn af(&self) -> &Register {
        &self.af
    }

    pub fn af_mut(&mut self) -> &mut Register {
        &mut self.af
    }

    pub fn bc(&self) -> &Register {
        &self.bc
    }

    pub fn bc_mut(&mut self) -> &mut Register {
        &mut self.bc
    }

    pub fn de(&self) -> &Register {
        &self.de
    }

    pub fn de_mut(&mut self) -> &mut Register {
        &mut self.de
    }

    pub fn hl(&self) -> &Register {
        &self.hl
    }

    pub fn hl_mut(&mut self) -> &mut Register {
        &mut self.hl
    }

    pub fn mem(&mut self) -> Arc<Mutex<Memory>> {
        self.mem.clone()
    }

    pub fn r16(&mut self, r: Option<Operand>) -> Result<&mut Register, &'static str> {
        match r {
            Some(o) => match o {
                Operand::BC => Ok(&mut self.bc),
                Operand::DE => Ok(&mut self.de),
                Operand::HL => Ok(&mut self.hl),
                Operand::AF => Ok(&mut self.af),
                Operand::SP => Ok(&mut self.sp),
                Operand::Mem(x) => Ok(self.r16(Some(*x))?),
                _ => Err("invalid R16"),
            },
            None => Err("missing Operand"),
        }
    }

    pub fn fetch(&mut self) -> Result<u8, MemoryError> {
        let mem = self.mem.lock().unwrap();
        let pc_val = self.pc.val();
        self.pc.inc();
        mem.read(pc_val)
    }

    pub fn fetch_word(&mut self) -> Result<u16, MemoryError> {
        let low = self.fetch()? as u16;
        let high = self.fetch()? as u16;
        Ok((high << 8) | low)
    }

    pub fn stop(&mut self) {
        self.stopped = true
    }

    pub fn halt(&mut self) {
        self.halted = true
    }

    fn decode(&self, opcode: u8) -> Result<Instruction, InstructionError> {
        instructions::decode(opcode)
    }

    //fn execute(&mut self, i: &Instruction) -> Result<(), Error> {
    //    Ok(())
    //}

    //fn execute_add(&mut self, i: &Instruction) -> Result<(), Error> {
    //    let dest = match i.dest() {
    //        Some(d) => d,
    //        None => return Err(Error::MissingDestination),
    //    };
    //    let src = match i.src() {
    //        Some(s) => s,
    //        None => return Err(Error::MissingSource),
    //    };
    //    Ok(())
    //}

    fn src_imm8(&mut self) -> Result<OperandValue, Error> {
        match self.fetch() {
            Ok(b) => Ok(OperandValue::Byte(b)),
            Err(e) => Err(Error::MemoryError(e)),
        }
    }

    fn src_imm16(&mut self) -> Result<OperandValue, Error> {
        match self.fetch_word() {
            Ok(w) => Ok(OperandValue::Word(w)),
            Err(e) => Err(Error::MemoryError(e)),
        }
    }
}

fn operand_type_match(a: OperandValue, b: OperandValue) -> bool {
    match a {
        OperandValue::Byte(_) => match b {
            OperandValue::Byte(_) => true,
            OperandValue::Word(_) => false,
        },
        OperandValue::Word(_) => match b {
            OperandValue::Byte(_) => false,
            OperandValue::Word(_) => true,
        },
    }
}

enum OperandValue {
    Byte(u8),
    Word(u16),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fetch_word() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        mem.lock().unwrap().write(0x00, 0xCC).unwrap();
        mem.lock().unwrap().write(0x01, 0xDD).unwrap();
        let expected_word = 0xDDCC;
        let mut cpu = CPU::new(mem);
        let fetched = cpu.fetch_word().unwrap();
        println!("{:x}", fetched);
        assert_eq!(expected_word, fetched);
    }
}
