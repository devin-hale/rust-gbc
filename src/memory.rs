use std::sync::{Arc, Mutex, MutexGuard};

use crate::{cpu::Interrupt, utils::bit};

const ROM_BANK_0_START: usize = 0x0000;
const ROM_BANK_0_END: usize = 0x03FFF;
const ROM_BANK_0_LEN: usize = ROM_BANK_0_END + 1;

const ROM_BANK_1_START: usize = 0x4000;
const ROM_BANK_1_END: usize = 0x7FFF;
const ROM_BANK_1_LEN: usize = ROM_BANK_1_END - ROM_BANK_1_START + 1;

const VRAM_START: usize = 0x8000;
const VRAM_END: usize = 0x9FFF;
const VRAM_LEN: usize = VRAM_END - VRAM_START + 1;

const ERAM_START: usize = 0xA000;
const ERAM_END: usize = 0xBFFF;
const ERAM_LEN: usize = (ERAM_END - ERAM_START) + 1;

const WRAM_0_START: usize = 0xC000;
const WRAM_0_END: usize = 0xCFFF;
const WRAM_0_LEN: usize = WRAM_0_END - WRAM_0_START + 1;

const WRAM_1_START: usize = 0xD000;
const WRAM_1_END: usize = 0xDFFF;
const WRAM_1_LEN: usize = WRAM_1_END - WRAM_0_START + 1;

const ECHO_RAM_START: usize = 0xE000;
const ECHO_RAM_END: usize = 0xFDFF;
const ECHO_RAM_LEN: usize = ECHO_RAM_END - ECHO_RAM_START + 1;

const OAM_START: usize = 0xFE00;
const OAM_END: usize = 0xFE9F;
const OAM_LEN: usize = OAM_END - OAM_START + 1;

const UNUSED_START: usize = 0xFEA0;
const UNUSED_END: usize = 0xFEFF;
const UNUSED_LEN: usize = UNUSED_END - UNUSED_START + 1;

const IO_START: usize = 0xFF00;
pub const DIV: usize = 0xFF04;
const TIMER_COUNTER: usize = 0xFF05;
const TIMER_MODULO: usize = 0xFF05;
const TIMER_CONTROL: usize = 0xFF07;
const IF_REGISTER: usize = 0xFF0F;
const LCDC: usize = 0xFF40;
const OBP0: usize = 0xFF48;
const OBP1: usize = 0xFF49;
const BGP: usize = 0xFF47;
const IO_END: usize = 0xFF7F;
const IO_LEN: usize = IO_END - IO_START + 1;

const HRAM_START: usize = 0xFF80;
const HRAM_END: usize = 0xFFFE;
const HRAM_LEN: usize = HRAM_END - HRAM_START + 1;

const IE_REGISTER: usize = 0xFFFF;

#[derive(Clone)]
pub struct Memory {
    rom_0: Arc<Mutex<[u8; ROM_BANK_0_LEN]>>,
    rom_1: Arc<Mutex<[u8; ROM_BANK_1_LEN]>>,
    vram: Arc<Mutex<[u8; ERAM_LEN]>>,
    eram: Arc<Mutex<[u8; ERAM_LEN]>>,
    io: Arc<Mutex<[u8; IO_LEN]>>,
    hram: Arc<Mutex<[u8; HRAM_LEN]>>,
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            rom_0: Arc::new(Mutex::new([0u8; ROM_BANK_0_LEN])),
            rom_1: Arc::new(Mutex::new([0u8; ROM_BANK_1_LEN])),
            vram: Arc::new(Mutex::new([0u8; VRAM_LEN])),
            eram: Arc::new(Mutex::new([0u8; ERAM_LEN])),
            io: Arc::new(Mutex::new([0u8; IO_LEN])),
            hram: Arc::new(Mutex::new([0u8; HRAM_LEN])),
        }
    }

    pub fn init(&mut self) {
        for av in DMG_INIT {
            self.write(av.0 as u16, av.1);
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        match addr {
            ROM_BANK_0_START..=ROM_BANK_0_END => self.rom_0()[addr],
            IO_START..=IO_END => self.io()[addr - IO_START],
            HRAM_START..=HRAM_END => self.io()[addr - HRAM_START],
            _ => {
                todo!("missing rom bank for addr: {}", addr);
            }
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        let addr = addr as usize;
        match addr {
            ROM_BANK_0_START..=ROM_BANK_0_END => self.rom_0()[addr] = val,
            IO_START..=IO_END => self.io()[addr - IO_START] = val,
            HRAM_START..=HRAM_END => self.io()[addr - HRAM_START] = val,
            _ => {
                todo!("missing rom bank for addr: {:x}", addr);
            }
        }
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        let low = self.read(addr);
        let high = self.read(addr + 1);
        ((high as u16) << 8) | low as u16
    }

    pub fn write_word(&mut self, addr: u16, data: u16) {
        let mut low = (data & 0x00FF) as u8;
        let mut high = ((data & 0xFF00) >> 8) as u8;

        if addr as usize == DIV {
            low = 0;
        }
        if (addr as usize) + 1 == DIV {
            high = 0;
        }

        self.write(addr, low);
        self.write(addr + 1, high);
    }

    pub fn inc(&mut self, addr: u16) -> u8 {
        let mut val = self.read(addr);
        val += 1;
        self.write(addr, val);
        val
    }

    pub fn dec(&mut self, addr: u16) -> u8 {
        if addr as usize == DIV {
            self.write(addr, 0);
            return 0;
        }
        let mut val = self.read(addr);
        val -= 1;
        self.write(addr, val);
        val
    }

    pub fn state(&self) -> Vec<[u16; 2]> {
        let mut mem_state: Vec<[u16; 2]> = vec![];

        for (i, v) in self.rom_0().iter().enumerate() {
            if *v != 0 {
                mem_state.push([i as u16, *v as u16]);
            }
        }

        for (i, v) in self.rom_1().iter().enumerate() {
            if *v != 0 {
                mem_state.push([i as u16, *v as u16]);
            }
        }

        for (i, v) in self.eram().iter().enumerate() {
            if *v != 0 {
                mem_state.push([i as u16, *v as u16]);
            }
        }
        mem_state
    }

    fn rom_0(&self) -> MutexGuard<'_, [u8; ROM_BANK_0_LEN]> {
        self.rom_0
            .lock()
            .expect("error acquiring mutex lock for rom bank 0")
    }

    fn rom_1(&self) -> MutexGuard<'_, [u8; ROM_BANK_1_LEN]> {
        self.rom_1
            .lock()
            .expect("error acquiring mutex lock for rom bank 1")
    }

    fn eram(&self) -> MutexGuard<'_, [u8; ERAM_LEN]> {
        self.eram
            .lock()
            .expect("error acquiring mutex lock for eram bank")
    }

    pub fn vram(&self) -> MutexGuard<'_, [u8; VRAM_LEN]> {
        self.vram
            .lock()
            .expect("error acquiring mutex lock for vram bank")
    }

    pub fn io(&self) -> MutexGuard<'_, [u8; IO_LEN]> {
        self.io
            .lock()
            .expect("error acquiring mutex lock for io bank")
    }

    pub fn hram(&self) -> MutexGuard<'_, [u8; IO_LEN]> {
        self.io
            .lock()
            .expect("error acquiring mutex lock for hram bank")
    }

    //pub fn interrupt_enable<'i>(&'i mut self) -> InterruptRegister<'i> {
    //    InterruptRegister(&mut self.read(IE_REGISTER as u16))
    //}

    //pub fn interrupt_flags<'i>(&'i mut self) -> InterruptRegister<'i> {
    //    InterruptRegister(&mut self.read(IF_REGISTER as u16))
    //}

    //pub fn inc_div(&mut self) {
    //    let mut div = self.div();
    //    *div = div.wrapping_add(1);
    //}

    //pub fn reset_div(&mut self) {
    //    *(self.div()) = 0;
    //}

    //pub fn timer_control<'t>(&'t mut self) -> TimerControl<'t> {
    //    TimerControl(&mut self.m[TIMER_CONTROL])
    //}

    //pub fn inc_timer(&mut self) {
    //    let val = self.m[TIMER_COUNTER];
    //    match val.checked_add(1) {
    //        Some(v) => self.m[TIMER_COUNTER] = v,
    //        None => {
    //            self.m[TIMER_COUNTER] = self.tma();
    //            self.interrupt_flags().timer_set();
    //        }
    //    }
    //}

    pub fn load_rom(&mut self, rom: &[u8]) {
        for i in 0..ROM_BANK_0_LEN {
            self.write(i as u16, rom[i]);
        }
    }

    //pub fn tma(&self) -> u8 {
    //    self.m[TIMER_MODULO]
    //}

    //pub fn lcdc(&self) -> u8 {
    //    self.m[LCDC]
    //}

    //pub fn bg_pal(&self) -> u8 {
    //    self.m[BGP]
    //}

    //pub fn obj_pal_0(&self) -> u8 {
    //    self.m[OBP0]
    //}

    //pub fn obj_pal_1(&self) -> u8 {
    //    self.m[OBP1]
    //}
}

pub struct IO<'a>(&'a mut [u8]);

impl<'a> IO<'a> {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

const DMG_INIT: [(usize, u8); 40] = [
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_write() {
        let mut mem = Memory::new();
        let addr = 0xFFEE;
        let byte = 0xDE;
        mem.write(addr, byte);
        assert_eq!(mem.read(addr), byte);
    }

    //#[test]
    //fn write_word() {
    //    let mut mem = Memory::new();
    //    let word = 0xFFEE;
    //    let low = 0xEE;
    //    let high = 0xFF;
    //    mem.write_word(0, word);
    //    assert_eq!(mem.read(0x00), low);
    //    assert_eq!(mem.read(0x01), high);
    //}

    //#[test]
    //fn read_word() {
    //    let mut mem = Memory::new();
    //    let word = 0xFFEE;
    //    mem.write_word(0, word);
    //    assert_eq!(mem.read_word(0x00), word);
    //}

    //#[test]
    //fn inc() {
    //    let mut mem = Memory::new();
    //    let addr = 0xFFEE;
    //    let byte = 0xDE;
    //    mem.m[addr as usize] = byte;
    //    mem.inc(addr);
    //    assert_eq!(mem.read(addr), byte + 1);
    //}

    //#[test]
    //fn dec() {
    //    let mut mem = Memory::new();
    //    let addr = 0xFFEE;
    //    let byte = 0xDE;
    //    mem.m[addr as usize] = byte;
    //    mem.dec(addr);
    //    assert_eq!(mem.read(addr), byte - 1);
    //}

    //#[test]
    //fn mem_io() {
    //    let mut mem = Memory::new();
    //    let io = mem.io();
    //    let len = IO_END - IO_START;
    //    assert_eq!(io.len(), len);
    //}

    //#[test]
    //fn timer_control() {
    //    let mut mem = Memory::new();
    //    mem.init();
    //    let mut cpy = mem.m[TIMER_CONTROL];
    //    let mut tc = TimerControl(&mut cpy);
    //    let mut mc = mem.timer_control();
    //    assert_eq!(tc.clk(), mc.clk());

    //    tc.enable();
    //    mc.enable();
    //    assert_eq!(tc.is_enabled(), mc.is_enabled());

    //    tc.disable();
    //    mc.disable();
    //    assert_eq!(tc.is_enabled(), mc.is_enabled());

    //    tc.clk_select(ClockSelect::Fast);
    //    mc.clk_select(ClockSelect::Fast);
    //    assert_eq!(tc.clk(), mc.clk());

    //    mc.clk_select(ClockSelect::Hyper);
    //    assert_ne!(tc.clk(), mc.clk());
    //}

    //#[test]
    //fn timer_inc() {
    //    let mut mem = Memory::new();
    //    mem.init();
    //    assert!(!mem.interrupt_flags().timer());
    //    mem.inc_timer();
    //    assert!(!mem.interrupt_flags().timer());
    //    mem.m[TIMER_COUNTER] = 255;
    //    mem.inc_timer();
    //    assert!(mem.interrupt_flags().timer());
    //    assert_eq!(mem.m[TIMER_COUNTER], mem.tma());
    //}
}
