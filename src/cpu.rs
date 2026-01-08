use std::sync::{Arc, Mutex};

use thiserror::Error;

use crate::{
    bit,
    instructions::{self, Cond, Instruction, InstructionError, Operand},
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
    interrupts_enabled: bool,
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

#[derive(Clone, Copy)]
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
            interrupts_enabled: false,
        }
    }

    pub fn flags(&self) -> u8 {
        self.af().low()
    }

    pub fn read_byte(&self, r: Option<Operand>) -> Result<u8, &'static str> {
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

    pub fn flag_set_val(&mut self, f: Flag, v: u8) {
        let v = v & 1;
        if v == 1 {
            self.flag_set(f);
        } else {
            self.flag_reset(f);
        }
    }

    pub fn flag_invert(&mut self, f: Flag) {
        if self.flag_is_set(f) {
            self.flag_reset(f);
        } else {
            self.flag_set(f);
        }
    }

    pub fn flag_reset(&mut self, f: Flag) {
        let mut flags = self.af.low();
        bit::reset(&mut flags, f as u8);
        self.af.write_byte(Level::Low, flags);
    }

    pub fn flag_is_set(&self, f: Flag) -> bool {
        self.flag(f) == 1
    }

    pub fn flag(&self, f: Flag) -> u8 {
        let flags = self.af.low();
        bit::get(flags, f as u8)
    }

    pub fn clear_flags(&mut self) {
        self.flag_reset(Flag::Z);
        self.flag_reset(Flag::C);
        self.flag_reset(Flag::HC);
        self.flag_reset(Flag::N);
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
        let data = self.read_byte(r.clone())? + 1;
        self.write_byte(r.clone(), data)?;
        Ok(data)
    }

    pub fn dec_byte(&mut self, r: Option<Operand>) -> Result<u8, &'static str> {
        let data = self.read_byte(r.clone())? - 1;
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

    pub fn write(&mut self, r: Option<Operand>, data: u16) -> Result<(), &'static str> {
        match r {
            Some(o) => match o {
                Operand::BC => self.bc.write(data),
                Operand::DE => self.de.write(data),
                Operand::HL => self.hl.write(data),
                Operand::AF => self.hl.write(data),
                Operand::SP => self.hl.write(data),
                Operand::Mem(x) => {
                    let addr = self.read(Some(*x))?;
                    let mut mem = self.mem.lock().unwrap();
                    mem.write_word(addr, data).unwrap();
                }
                _ => return Err("invalid R16"),
            },
            None => return Err("missing Operand"),
        }
        Ok(())
    }

    pub fn read(&mut self, r: Option<Operand>) -> Result<u16, &'static str> {
        match r {
            Some(o) => match o {
                Operand::BC => Ok(self.bc.val()),
                Operand::DE => Ok(self.de.val()),
                Operand::HL => Ok(self.hl.val()),
                Operand::AF => Ok(self.af.val()),
                Operand::SP => Ok(self.sp.val()),
                Operand::Mem(x) => {
                    let addr = self.read(Some(*x))?;
                    let mem = self.mem.lock().unwrap();
                    Ok(mem.read_word(addr).unwrap())
                }
                _ => Err("invalid R16"),
            },
            None => Err("missing Operand"),
        }
    }

    pub fn register(&mut self, r: Option<Operand>) -> Result<&mut Register, &'static str> {
        match r {
            Some(o) => match o {
                Operand::BC => Ok(&mut self.bc),
                Operand::DE => Ok(&mut self.de),
                Operand::HL => Ok(&mut self.hl),
                Operand::AF => Ok(&mut self.af),
                Operand::SP => Ok(&mut self.sp),
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

    pub fn jump_relative(&mut self, b: u8) -> Result<u16, &'static str> {
        let addr = self.pc.val() + (b as u16);
        self.pc.write(addr);
        Ok(addr)
    }

    pub fn jump_relative_word(&mut self, b: u16) -> Result<u16, &'static str> {
        let addr = self.pc.val() + b;
        self.pc.write(addr);
        Ok(addr)
    }

    pub fn cc(&self, c: Option<Operand>) -> Result<bool, &'static str> {
        match c {
            Some(o) => match o {
                Operand::Cond(c) => match c {
                    Cond::Z => Ok(self.flag_is_set(Flag::Z)),
                    Cond::NZ => Ok(!self.flag_is_set(Flag::Z)),
                    Cond::C => Ok(self.flag_is_set(Flag::C)),
                    Cond::NC => Ok(!self.flag_is_set(Flag::C)),
                },
                _ => Err("incorrect operand type"),
            },
            None => Err("missing operand"),
        }
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fetch() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        mem.lock().unwrap().write(0x00, 0xCC).unwrap();
        let expected = 0xCC;

        let mut cpu = CPU::new(mem);
        let fetched = cpu.fetch().unwrap();
        assert_eq!(expected, fetched);
        assert_eq!(cpu.pc.val(), 0x1);
    }

    #[test]
    fn fetch_word() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        mem.lock().unwrap().write(0x00, 0xCC).unwrap();
        mem.lock().unwrap().write(0x01, 0xDD).unwrap();
        let expected_word = 0xDDCC;
        let mut cpu = CPU::new(mem);
        let fetched = cpu.fetch_word().unwrap();
        assert_eq!(expected_word, fetched);
        assert_eq!(cpu.pc.val(), 0x2);
    }

    #[test]
    fn jump_relative() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(mem);
        let byte = 0xFF;
        let expected = cpu.pc.val() + (byte as u16);
        cpu.jump_relative(byte).unwrap();
        assert_eq!(cpu.pc.val(), expected);
    }

    #[test]
    fn jump_relative_word() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(mem);
        let word = 0xFF00;
        let expected = cpu.pc.val() + word;
        cpu.jump_relative_word(word).unwrap();
        assert_eq!(cpu.pc.val(), expected);
    }

    #[test]
    fn flags() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(mem);
        let byte = 0b0110_0110;
        cpu.af.write_byte(Level::Low, byte);
        assert_eq!(cpu.flags(), byte);
    }

    #[test]
    fn flag_set() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(mem);
        assert_ne!(bit::get(cpu.af.low(), Flag::N as u8), 1);
        cpu.flag_set(Flag::N);
        assert_eq!(bit::get(cpu.af.low(), Flag::N as u8), 1);
    }

    #[test]
    fn flag_reset() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(mem);
        cpu.flag_set(Flag::N);
        assert_eq!(bit::get(cpu.af.low(), Flag::N as u8), 1);
        cpu.flag_reset(Flag::N);
        assert_eq!(bit::get(cpu.af.low(), Flag::N as u8), 0);
    }

    #[test]
    fn flag_is_set() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let cpu = CPU::new(mem);
        assert!(!bit::is_set(cpu.flags(), Flag::N as u8));
    }

    #[test]
    fn flag_set_val() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(mem);
        let f = Flag::C;
        cpu.flag_set_val(f, 1);
        assert!(cpu.flag_is_set(f));
        cpu.flag_set_val(f, 0);
        assert!(!cpu.flag_is_set(f));
    }

    #[test]
    fn hlmem_byte() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(mem);

        let addr = 0xFFEE;
        let data = 0x99;
        let op = Operand::Mem(Box::new(Operand::HL));

        cpu.mem.lock().unwrap().write(addr, data).unwrap();
        cpu.write(Some(Operand::HL), addr).unwrap();

        let from_cpu = cpu.read_byte(Some(op)).unwrap();
        assert_eq!(from_cpu, data);
    }

    #[test]
    fn hlmem_word() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(mem);

        let addr = 0xFFEE;
        let data = 0x9900;
        let op = Operand::Mem(Box::new(Operand::HL));

        cpu.mem.lock().unwrap().write_word(addr, data).unwrap();
        cpu.write(Some(Operand::HL), addr).unwrap();

        let from_cpu = cpu.read(Some(op)).unwrap();
        assert_eq!(from_cpu, data);
    }

    #[test]
    fn inc_byte() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(mem);
        let expect = cpu.bc.high() + 1;
        cpu.inc_byte(Some(Operand::B)).unwrap();

        assert_eq!(cpu.bc.high(), expect);
    }

    #[test]
    fn dec_byte() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(mem);
        cpu.bc.write_byte(Level::High, 0xFF);
        let expect = cpu.bc.high() - 1;
        cpu.dec_byte(Some(Operand::B)).unwrap();
        assert_eq!(cpu.bc.high(), expect);
    }

    #[test]
    fn cc() {
        let mem = Arc::new(Mutex::new(Memory::new()));
        let mut cpu = CPU::new(mem);
        assert!(cpu.cc(Some(Operand::Cond(Cond::NZ))).unwrap());
        assert!(cpu.cc(Some(Operand::Cond(Cond::NC))).unwrap());
        cpu.flag_set(Flag::Z);
        cpu.flag_set(Flag::C);
        assert!(cpu.cc(Some(Operand::Cond(Cond::Z))).unwrap());
        assert!(cpu.cc(Some(Operand::Cond(Cond::C))).unwrap());
    }
}
