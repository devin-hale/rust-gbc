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

pub enum IFlag {
    VBlank,
    LCD,
    Timer,
    Serial,
    Joypad,
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

    fn is_set(&self, f: IFlag) -> bool {
        self.flag(f) != 0
    }

    fn set_flag(&mut self, f: IFlag) {
        let mut r = self.register();
        bit::set(&mut r, f as u8);
        self.set_register(r);
    }

    fn reset_flag(&mut self, f: IFlag) {
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
}

pub struct TimerControl<'i>(&'i mut u8);

impl<'t> TimerControl<'t> {
    const ENABLE_BIT: u8 = 2;
    const CLOCK_SELECT_MASK: u8 = 0b11;

    pub fn enable(&mut self) {
        bit::set(self.0, Self::ENABLE_BIT);
    }

    pub fn disable(&mut self) {
        bit::reset(self.0, Self::ENABLE_BIT);
    }

    pub fn is_enabled(&mut self) -> bool {
        bit::is_set(*self.0, Self::ENABLE_BIT)
    }

    pub fn clk(&mut self) -> ClockSelect {
        (*self.0 & Self::CLOCK_SELECT_MASK).into()
    }

    pub fn clk_select(&mut self, cs: ClockSelect) {
        *self.0 = (*self.0 & !Self::CLOCK_SELECT_MASK) | cs as u8;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockSelect {
    Hyper,
    Slow,
    Medium,
    Fast,
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

impl ClockSelect {
    pub fn cycles(&self) -> u64 {
        match self {
            ClockSelect::Hyper => 256_000_000,
            ClockSelect::Slow => 4_000_000,
            ClockSelect::Medium => 16_000_000,
            ClockSelect::Fast => 64_000_000,
        }
    }
}
