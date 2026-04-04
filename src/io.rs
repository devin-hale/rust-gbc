use std::time::Instant;

use crate::{memory::Memory, utils::bit};

pub const DIV: usize = 0xFF04;
const TIMER_COUNTER: usize = 0xFF05;
const TIMER_MODULO: usize = 0xFF05;
const TIMER_CONTROL: usize = 0xFF07;
const IF_REGISTER: usize = 0xFF0F;
const LCDC: usize = 0xFF40;
const OBP0: usize = 0xFF48;
const OBP1: usize = 0xFF49;
const BGP: usize = 0xFF47;

#[derive(Clone, Copy)]
pub enum IFlag {
    VBlank,
    LCD,
    Timer,
    Serial,
    Joypad,
}

impl IFlag {
    pub fn addr(&self) -> u16 {
        match self {
            Self::VBlank => 0x40,
            Self::LCD => 0x48,
            Self::Timer => 0x50,
            Self::Serial => 0x58,
            Self::Joypad => 0x60,
        }
    }
}

pub enum IRType {
    IE,
    IF,
}

pub struct IR {
    ir_type: IRType,
    mem: Memory,
}

impl IR {
    pub fn new(ir_type: IRType, mem: Memory) -> Self {
        Self { ir_type, mem }
    }

    fn register(&self) -> u8 {
        match self.ir_type {
            IRType::IE => *self.mem.ie(),
            IRType::IF => self.mem.read(0xFF0F),
        }
    }

    fn set_register(&mut self, val: u8) {
        match self.ir_type {
            IRType::IE => *self.mem.ie() = val,
            IRType::IF => self.mem.write(0xFF0F, val),
        }
    }

    pub fn flag(&self, f: IFlag) -> u8 {
        let r = self.register();
        bit::get(r, f as u8)
    }

    pub fn is_set(&self, f: IFlag) -> bool {
        self.flag(f) != 0
    }

    pub fn set_flag(&mut self, f: IFlag) {
        let mut r = self.register();
        bit::set(&mut r, f as u8);
        self.set_register(r);
    }

    pub fn reset_flag(&mut self, f: IFlag) {
        let mut r = self.register();
        bit::reset(&mut r, f as u8);
        self.set_register(r);
    }

    fn set_flag_from_val(&mut self, f: IFlag, val: u8) {
        match val {
            0 => self.set_flag(f),
            _ => self.reset_flag(f),
        }
    }

    // returns highest priority flag that is active
    pub fn active(&self) -> Option<IFlag> {
        if self.is_set(IFlag::VBlank) {
            Some(IFlag::VBlank)
        } else if self.is_set(IFlag::LCD) {
            Some(IFlag::LCD)
        } else if self.is_set(IFlag::Timer) {
            Some(IFlag::Timer)
        } else if self.is_set(IFlag::Serial) {
            Some(IFlag::Serial)
        } else if self.is_set(IFlag::Joypad) {
            Some(IFlag::Joypad)
        } else {
            None
        }
    }
}

pub enum Timer {
    DIV = 0xFF04,
    TIMA,
    TMA,
    TAC,
}

pub struct DivRegister {
    mem: Memory,
    last_inc: Instant,
    disabled: bool,
}

impl DivRegister {
    const FREQ: u128 = 16_384; // Hz
    const PRD_NS: u128 = (1 * 1000 * 1000 * 1000) / Self::FREQ; // ns

    pub fn new(mem: Memory, start: Instant) -> Self {
        Self {
            mem,
            last_inc: start,
            disabled: false,
        }
    }

    pub fn enable(&mut self) {
        self.disabled = false;
    }

    pub fn disable(&mut self) {
        self.disabled = true;
    }

    pub fn advance_to(&mut self, t: Instant) {
        if !self.disabled {
            let since_last = t.checked_duration_since(self.last_inc).unwrap().as_nanos();
            let ticks = (since_last / Self::PRD_NS) as u8;

            let val = self.mem.read(DIV as u16);
            self.mem.write(DIV as u16, val.wrapping_add(ticks));
        }
    }
}

pub struct TimerControl(Memory);

impl TimerControl {
    const ADDR: u16 = 0xFF07;
    const ENABLE_BIT: u8 = 2;
    const CLOCK_SELECT_MASK: u8 = 0b11;

    fn val(&self) -> u8 {
        self.0.read(Self::ADDR)
    }

    fn set(&mut self, val: u8) {
        self.0.write(Self::ADDR, val);
    }

    pub fn enable(&mut self) {
        let mut val = self.val();
        bit::set(&mut val, Self::ENABLE_BIT);
        self.set(val);
    }

    pub fn disable(&mut self) {
        let mut val = self.val();
        bit::reset(&mut val, Self::ENABLE_BIT);
        self.set(val);
    }

    pub fn is_enabled(&mut self) -> bool {
        bit::is_set(self.val(), Self::ENABLE_BIT)
    }

    pub fn clk(&self) -> ClockSelect {
        (self.val() & Self::CLOCK_SELECT_MASK).into()
    }

    pub fn clk_select(&mut self, cs: ClockSelect) {
        let val = (self.val() & !Self::CLOCK_SELECT_MASK) | cs as u8;
        self.set(val);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockSelect {
    Hyper = 256_000_000,
    Slow = 4_000_000,
    Medium = 16_000_000,
    Fast = 64_000_000,
}

impl From<u8> for ClockSelect {
    fn from(value: u8) -> Self {
        match value {
            0 => ClockSelect::Hyper,
            1 => ClockSelect::Slow,
            2 => ClockSelect::Medium,
            3 => ClockSelect::Fast,
            _ => panic!("invalid ClockSelect value"),
        }
    }
}
