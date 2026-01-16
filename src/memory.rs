use std::sync::{Arc, Mutex};

use crate::utils::bit;

const ROM_BANK_0_START: u16 = 0x0000;
const ROM_BANK_0_END: u16 = 0x03FFF;

const ROM_BANK_1_START: u16 = 0x4000;
const ROM_BANK_1_END: u16 = 0x7FFF;

const VRAM_START: u16 = 0x8000;
const VRAM_END: u16 = 0x9FFF;

const ERAM_START: u16 = 0xA000;
const ERAM_END: u16 = 0xBFFF;

const WRAM_0_START: u16 = 0xC000;
const WRAM_0_END: u16 = 0xCFFF;

const WRAM_1_START: u16 = 0xD000;
const WRAM_1_END: u16 = 0xDFFF;

const ECHO_RAM_START: u16 = 0xE000;
const ECHO_RAM_END: u16 = 0xFDFF;

const OAM_START: u16 = 0xFE00;
const OAM_END: u16 = 0xFE9F;

const UNUSED_START: u16 = 0xFEA0;
const UNUSED_END: u16 = 0xFEFF;

const IO_START: u16 = 0xFF00;
pub const DIV: u16 = 0xFF04;
const TIMER_CONTROL: u16 = 0xFF04;
const IF_REGISTER: u16 = 0xFF0F;
const IO_END: u16 = 0xFF7F;

const HRAM_START: u16 = 0xFF80;
const HRAM_END: u16 = 0xFFFE;

const IE_REGISTER: u16 = 0xFFFF;

pub struct Memory {
    m: [u8; 0x1_0000],
}

pub struct IO<'a>(&'a mut [u8]);

impl<'a> IO<'a> {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

const DMG_INIT: [(u16, u8); 40] = [
    (0xFF00, 0xCF),
    (0xFF01, 0x00),
    (0xFF02, 0x7E),
    (0xFF04, 0xAB),
    (0xFF05, 0x00),
    (0xFF06, 0x00),
    (0xFF07, 0xF8),
    (0xFF0F, 0xE1),
    (0xFF10, 0x80),
    (0xFF11, 0xBF),
    (0xFF12, 0xF3),
    (0xFF13, 0xFF),
    (0xFF14, 0xBF),
    (0xFF16, 0x3F),
    (0xFF17, 0x00),
    (0xFF18, 0xFF),
    (0xFF19, 0xBF),
    (0xFF1A, 0x7F),
    (0xFF1B, 0xFF),
    (0xFF1C, 0x9F),
    (0xFF1D, 0xFF),
    (0xFF1E, 0xBF),
    (0xFF20, 0xFF),
    (0xFF21, 0x00),
    (0xFF22, 0x00),
    (0xFF23, 0xBF),
    (0xFF24, 0x77),
    (0xFF25, 0xF3),
    (0xFF26, 0xF1),
    (0xFF40, 0x91),
    (0xFF41, 0x85),
    (0xFF42, 0x00),
    (0xFF43, 0x00),
    (0xFF44, 0x00),
    (0xFF45, 0x00),
    (0xFF46, 0xFF),
    (0xFF47, 0xFC),
    (0xFF4A, 0x00),
    (0xFF4B, 0x00),
    (0xFFFF, 0x00),
];

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
    fn cycles(&self) -> u64 {
        match self {
            ClockSelect::Hyper => 256_000_000,
            ClockSelect::Slow => 4_000_000,
            ClockSelect::Medium => 16_000_000,
            ClockSelect::Fast => 64_000_000,
        }
    }
}

impl Memory {
    pub fn new() -> Memory {
        Memory { m: [0u8; 0x1_0000] }
    }

    pub fn arc() -> Arc<Mutex<Memory>> {
        Arc::new(Mutex::new(Memory::new()))
    }

    pub fn init(&mut self) {
        for av in DMG_INIT {
            self.m[av.0 as usize] = av.1;
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.m[addr as usize]
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        let low = self.m[addr as usize];
        let high = self.m[(addr + 1) as usize];
        ((high as u16) << 8) | low as u16
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        let mut d = data;
        if addr == DIV {
            d = 0;
        }
        self.m[addr as usize] = d;
    }

    pub fn write_word(&mut self, addr: u16, data: u16) {
        let mut low = (data & 0x00FF) as u8;
        let mut high = ((data & 0xFF00) >> 8) as u8;

        if addr == DIV {
            low = 0;
        }
        if addr + 1 == DIV {
            high = 0;
        }

        self.write(addr, low);
        self.write(addr + 1, high);
    }

    pub fn inc(&mut self, addr: u16) -> u8 {
        self.m[addr as usize] += 1;
        self.read(addr)
    }

    pub fn dec(&mut self, addr: u16) -> u8 {
        if addr == DIV {
            self.m[addr as usize] = 0;
            return 0;
        }
        self.m[addr as usize] -= 1;
        self.read(addr)
    }

    pub fn state(&self) -> Vec<State> {
        let mut mem_state: Vec<State> = vec![];
        for (i, v) in self.m.iter().enumerate() {
            if *v != 0 {
                mem_state.push(State {
                    addr: i as u16,
                    val: *v,
                })
            }
        }
        mem_state
    }

    pub fn check_state(&self, s: &State) -> bool {
        s.val == self.m[s.addr as usize]
    }

    pub fn compare_state(&self, mem_state: &[State]) -> bool {
        let local = self.state();
        for (a, b) in local.iter().zip(mem_state.iter()) {
            if a.addr != b.addr || a.val != b.val {
                return false;
            }
        }
        true
    }

    pub fn io<'i>(&'i mut self) -> IO<'i> {
        IO(&mut self.m[(IO_START as usize)..(IO_END as usize)])
    }

    pub fn interrupt_enable<'i>(&'i mut self) -> InterruptRegister<'i> {
        InterruptRegister(&mut self.m[IE_REGISTER as usize])
    }

    pub fn interrupt_flags<'i>(&'i mut self) -> InterruptRegister<'i> {
        InterruptRegister(&mut self.m[IF_REGISTER as usize])
    }

    pub fn div(&self) -> u8 {
        self.m[DIV as usize]
    }

    pub fn inc_div(&mut self) {
        let mut div = self.div();
        div = div.wrapping_add(1);
        self.m[DIV as usize] = div;
    }

    pub fn reset_div(&mut self) {
        self.m[DIV as usize] = 0;
    }

    pub fn timer_control<'t>(&'t mut self) -> TimerControl<'t> {
        TimerControl(&mut self.m[TIMER_CONTROL as usize])
    }
}

#[derive(PartialEq, Eq)]
pub struct State {
    addr: u16,
    val: u8,
}

impl State {
    fn addr(&self) -> u16 {
        self.addr
    }

    fn val(&self) -> u8 {
        self.val
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read() {
        let mut mem = Memory::new();
        let addr = 0xFFEE;
        let byte = 0xDE;
        mem.m[addr as usize] = byte;
        assert_eq!(mem.read(addr), byte);
    }

    #[test]
    fn write() {
        let mut mem = Memory::new();
        let addr = 0xFFEE;
        let byte = 0xDE;
        mem.write(addr, byte);
        assert_eq!(mem.m[addr as usize], byte);
    }

    #[test]
    fn write_word() {
        let mut mem = Memory::new();
        let word = 0xFFEE;
        let low = 0xEE;
        let high = 0xFF;
        mem.write_word(0, word);
        assert_eq!(mem.read(0x00), low);
        assert_eq!(mem.read(0x01), high);
    }

    #[test]
    fn read_word() {
        let mut mem = Memory::new();
        let word = 0xFFEE;
        mem.write_word(0, word);
        assert_eq!(mem.read_word(0x00), word);
    }

    #[test]
    fn inc() {
        let mut mem = Memory::new();
        let addr = 0xFFEE;
        let byte = 0xDE;
        mem.m[addr as usize] = byte;
        mem.inc(addr);
        assert_eq!(mem.read(addr), byte + 1);
    }

    #[test]
    fn dec() {
        let mut mem = Memory::new();
        let addr = 0xFFEE;
        let byte = 0xDE;
        mem.m[addr as usize] = byte;
        mem.dec(addr);
        assert_eq!(mem.read(addr), byte - 1);
    }

    #[test]
    fn mem_io() {
        let mut mem = Memory::new();
        let io = mem.io();
        let len = (IO_END - IO_START) as usize;
        assert_eq!(io.len(), len);
    }

    #[test]
    fn timer_control() {
        let mut mem = Memory::new();
        mem.init();
        let mut cpy = mem.m[TIMER_CONTROL as usize];
        let mut tc = TimerControl(&mut cpy);
        let mut mc = mem.timer_control();
        assert_eq!(tc.clk(), mc.clk());

        tc.enable();
        mc.enable();
        assert_eq!(tc.is_enabled(), mc.is_enabled());

        tc.disable();
        mc.disable();
        assert_eq!(tc.is_enabled(), mc.is_enabled());

        tc.clk_select(ClockSelect::Fast);
        mc.clk_select(ClockSelect::Fast);
        assert_eq!(tc.clk(), mc.clk());

        mc.clk_select(ClockSelect::Hyper);
        assert_ne!(tc.clk(), mc.clk());
    }
}
