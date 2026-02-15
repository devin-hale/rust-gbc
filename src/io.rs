use crate::{cpu::Interrupt, utils::bit};

pub const DIV: usize = 0xFF04;
const TIMER_COUNTER: usize = 0xFF05;
const TIMER_MODULO: usize = 0xFF05;
const TIMER_CONTROL: usize = 0xFF07;
const IF_REGISTER: usize = 0xFF0F;
const LCDC: usize = 0xFF40;
const OBP0: usize = 0xFF48;
const OBP1: usize = 0xFF49;
const BGP: usize = 0xFF47;

pub struct InterruptRegister<'i>(&'i mut u8);

impl<'i> InterruptRegister<'i> {
    pub fn vblank(&self) -> bool {
        bit::is_set(*self.0, 0)
    }

    #[inline(always)]
    pub fn vblank_set(&mut self) {
        bit::set(self.0, 0)
    }

    #[inline(always)]
    pub fn vblank_reset(&mut self) {
        bit::reset(self.0, 0)
    }

    #[inline(always)]
    pub fn lcd(&self) -> bool {
        bit::is_set(*self.0, 1)
    }

    #[inline(always)]
    pub fn lcd_set(&mut self) {
        bit::set(self.0, 1)
    }

    #[inline(always)]
    pub fn lcd_reset(&mut self) {
        bit::reset(self.0, 1)
    }

    #[inline(always)]
    pub fn timer(&self) -> bool {
        bit::is_set(*self.0, 2)
    }

    #[inline(always)]
    pub fn timer_set(&mut self) {
        bit::set(self.0, 2)
    }

    #[inline(always)]
    pub fn timer_reset(&mut self) {
        bit::reset(self.0, 2)
    }

    #[inline(always)]
    pub fn serial(&self) -> bool {
        bit::is_set(*self.0, 3)
    }

    #[inline(always)]
    pub fn serial_set(&mut self) {
        bit::set(self.0, 3)
    }

    #[inline(always)]
    pub fn serial_reset(&mut self) {
        bit::reset(self.0, 3)
    }

    #[inline(always)]
    pub fn joypad(&self) -> bool {
        bit::is_set(*self.0, 4)
    }

    #[inline(always)]
    pub fn joypad_set(&mut self) {
        bit::set(self.0, 4)
    }

    #[inline(always)]
    pub fn joypad_reset(&mut self) {
        bit::reset(self.0, 4)
    }

    pub fn reset(&mut self, i: Interrupt) {
        match i {
            Interrupt::VBlank => self.vblank_reset(),
            Interrupt::STAT => self.lcd_reset(),
            Interrupt::Serial => self.serial_reset(),
            Interrupt::Timer => self.timer_reset(),
            Interrupt::Joypad => self.joypad_reset(),
        }
    }

    pub fn set(&mut self, i: Interrupt) {
        match i {
            Interrupt::VBlank => self.vblank_set(),
            Interrupt::STAT => self.lcd_set(),
            Interrupt::Serial => self.serial_set(),
            Interrupt::Timer => self.timer_set(),
            Interrupt::Joypad => self.joypad_set(),
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
