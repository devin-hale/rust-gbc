use std::{
    fmt::Display,
    sync::{Arc, Mutex, MutexGuard},
};

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
    wram_0: Arc<Mutex<[u8; WRAM_0_LEN]>>,
    wram_1: Arc<Mutex<[u8; WRAM_1_LEN]>>,
    echo_ram: Arc<Mutex<[u8; ECHO_RAM_LEN]>>,
    oam: Arc<Mutex<[u8; OAM_LEN]>>,
    unused: Arc<Mutex<[u8; UNUSED_LEN]>>,
    io: Arc<Mutex<[u8; IO_LEN]>>,
    hram: Arc<Mutex<[u8; HRAM_LEN]>>,
    ie: Arc<Mutex<u8>>,

    addr: Arc<Mutex<u16>>,
    data: Arc<Mutex<u8>>,
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            rom_0: Arc::new(Mutex::new([0u8; ROM_BANK_0_LEN])),
            rom_1: Arc::new(Mutex::new([0u8; ROM_BANK_1_LEN])),
            vram: Arc::new(Mutex::new([0u8; VRAM_LEN])),
            eram: Arc::new(Mutex::new([0u8; ERAM_LEN])),
            wram_0: Arc::new(Mutex::new([0u8; WRAM_0_LEN])),
            wram_1: Arc::new(Mutex::new([0u8; WRAM_1_LEN])),
            echo_ram: Arc::new(Mutex::new([0u8; ECHO_RAM_LEN])),
            oam: Arc::new(Mutex::new([0u8; OAM_LEN])),
            unused: Arc::new(Mutex::new([0u8; UNUSED_LEN])),
            io: Arc::new(Mutex::new([0u8; IO_LEN])),
            hram: Arc::new(Mutex::new([0u8; HRAM_LEN])),
            ie: Arc::new(Mutex::new(0)),
            addr: Arc::new(Mutex::new(0)),
            data: Arc::new(Mutex::new(0)),
        }
    }

    pub fn data_bus(&self, a: Accessor) -> DataBus {
        DataBus {
            accessor: a,
            mem: self.clone(),
        }
    }

    pub fn address_bus(&self, a: Accessor) -> AddressBus {
        AddressBus {
            accessor: a,
            mem: self.clone(),
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
            ROM_BANK_1_START..=ROM_BANK_1_END => self.rom_1()[addr - ROM_BANK_1_START],
            VRAM_START..=VRAM_END => self.vram()[addr - VRAM_START],
            ERAM_START..=ERAM_END => self.eram()[addr - ERAM_START],
            WRAM_0_START..=WRAM_0_END => self.wram_0()[addr - WRAM_0_START],
            WRAM_1_START..=WRAM_1_END => self.wram_1()[addr - WRAM_1_START],
            ECHO_RAM_START..=ECHO_RAM_END => self.echo_ram()[addr - ECHO_RAM_START],
            OAM_START..=OAM_END => self.oam()[addr - OAM_START],
            UNUSED_START..=UNUSED_END => self.oam()[addr - UNUSED_START],
            IO_START..=IO_END => self.io()[addr - IO_START],
            HRAM_START..=HRAM_END => self.hram()[addr - HRAM_START],
            IE_REGISTER => *self.ie(),
            _ => {
                todo!("missing rom bank for addr: {}", addr);
            }
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        let addr = addr as usize;
        match addr {
            ROM_BANK_0_START..=ROM_BANK_0_END => self.rom_0()[addr] = val,
            ROM_BANK_1_START..=ROM_BANK_1_END => self.rom_1()[addr - ROM_BANK_1_START] = val,
            VRAM_START..=VRAM_END => self.vram()[addr - VRAM_START] = val,
            ERAM_START..=ERAM_END => self.eram()[addr - ERAM_START] = val,
            WRAM_0_START..=WRAM_0_END => self.wram_0()[addr - WRAM_0_START] = val,
            WRAM_1_START..=WRAM_1_END => self.wram_1()[addr - WRAM_1_START] = val,
            ECHO_RAM_START..=ECHO_RAM_END => self.echo_ram()[addr - ECHO_RAM_START] = val,
            OAM_START..=OAM_END => self.oam()[addr - OAM_START] = val,
            UNUSED_START..=UNUSED_END => self.oam()[addr - UNUSED_START] = val,
            IO_START..=IO_END => self.io()[addr - IO_START] = val,
            HRAM_START..=HRAM_END => self.hram()[addr - HRAM_START] = val,
            IE_REGISTER => *self.ie() = val,
            _ => {
                todo!("missing rom bank for addr: {:x}", addr);
            }
        }
    }

    fn addr(&self) -> MutexGuard<'_, u16> {
        self.addr
            .lock()
            .expect("error acquiring lock for address line")
    }

    fn data(&self) -> MutexGuard<'_, u8> {
        self.data
            .lock()
            .expect("error acquiring lock for data line")
    }

    pub fn read_current(&self) -> u8 {
        self.read(*self.addr())
    }

    pub fn write_current(&mut self) {
        let addr = *self.addr();
        let data = *self.data();
        self.write(addr, data);
    }

    pub fn assert_addr(&mut self, addr: u16) {
        *self.addr() = addr
    }

    pub fn assert_data(&mut self, data: u8) {
        *self.data() = data
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        let low = self.read(addr);
        let high = self.read(addr + 1);
        ((high as u16) << 8) | low as u16
    }

    pub fn write_word(&mut self, addr: u16, data: u16) {
        let low = (data & 0x00FF) as u8;
        let high = ((data & 0xFF00) >> 8) as u8;

        //if addr as usize == DIV {
        //    low = 0;
        //}
        //if (addr as usize) + 1 == DIV {
        //    high = 0;
        //}

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
        //if addr as usize == DIV {
        //    self.write(addr, 0);
        //    return 0;
        //}
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
                mem_state.push([(i + ROM_BANK_1_START) as u16, *v as u16]);
            }
        }

        for (i, v) in self.eram().iter().enumerate() {
            if *v != 0 {
                mem_state.push([(i + ERAM_START) as u16, *v as u16]);
            }
        }

        for (i, v) in self.wram_0().iter().enumerate() {
            if *v != 0 {
                mem_state.push([(i + WRAM_0_START) as u16, *v as u16]);
            }
        }

        for (i, v) in self.wram_1().iter().enumerate() {
            if *v != 0 {
                mem_state.push([(i + WRAM_1_START) as u16, *v as u16]);
            }
        }

        for (i, v) in self.echo_ram().iter().enumerate() {
            if *v != 0 {
                mem_state.push([(i + ECHO_RAM_START) as u16, *v as u16]);
            }
        }

        for (i, v) in self.oam().iter().enumerate() {
            if *v != 0 {
                mem_state.push([(i + OAM_START) as u16, *v as u16]);
            }
        }

        for (i, v) in self.vram().iter().enumerate() {
            if *v != 0 {
                mem_state.push([(i + VRAM_START) as u16, *v as u16]);
            }
        }

        for (i, v) in self.io().iter().enumerate() {
            if *v != 0 {
                mem_state.push([(i + IO_START) as u16, *v as u16]);
            }
        }

        for (i, v) in self.hram().iter().enumerate() {
            if *v != 0 {
                mem_state.push([(i + HRAM_START) as u16, *v as u16]);
            }
        }

        if *self.ie() != 0 {
            mem_state.push([IE_REGISTER as u16, *self.ie() as u16]);
        }
        mem_state
    }

    pub fn load_state(&mut self, s: &Vec<[u16; 2]>) {
        for state in s.iter() {
            let addr = state[0];
            let val = state[1] as u8;
            self.write(addr, val);
        }
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

    pub fn hram(&self) -> MutexGuard<'_, [u8; HRAM_LEN]> {
        self.hram
            .lock()
            .expect("error acquiring mutex lock for hram bank")
    }

    pub fn wram_0(&self) -> MutexGuard<'_, [u8; WRAM_0_LEN]> {
        self.wram_0
            .lock()
            .expect("error acquiring mutex lock for hram bank")
    }

    pub fn wram_1(&self) -> MutexGuard<'_, [u8; WRAM_1_LEN]> {
        self.wram_1
            .lock()
            .expect("error acquiring mutex lock for hram bank")
    }

    pub fn echo_ram(&self) -> MutexGuard<'_, [u8; ECHO_RAM_LEN]> {
        self.echo_ram
            .lock()
            .expect("error acquiring mutex lock for hram bank")
    }

    pub fn oam(&self) -> MutexGuard<'_, [u8; OAM_LEN]> {
        self.oam
            .lock()
            .expect("error acquiring mutex lock for hram bank")
    }

    pub fn unused(&self) -> MutexGuard<'_, [u8; UNUSED_LEN]> {
        self.unused
            .lock()
            .expect("error acquiring mutex lock for hram bank")
    }

    pub fn ie(&self) -> MutexGuard<'_, u8> {
        self.ie
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

pub struct DataBus {
    accessor: Accessor,
    mem: Memory,
}

impl DataBus {
    pub fn read(&self) -> u8 {
        self.mem.read_current()
    }

    pub fn assert(&mut self, data: u8) {
        self.mem.assert_data(data);
    }

    pub fn write(&mut self) {
        self.mem.write_current();
    }
}

pub struct AddressBus {
    accessor: Accessor,
    mem: Memory,
}

impl AddressBus {
    pub fn assert(&mut self, addr: u16) {
        self.mem.assert_addr(addr);
    }

    pub fn current(&self) -> u16 {
        *self.mem.addr()
    }
}

#[derive(Debug, Clone)]
pub enum Accessor {
    CPU,
    PPU,
}

impl Display for Accessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::CPU => "cpu",
            Self::PPU => "ppu",
        };
        write!(f, "{}", s)
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
