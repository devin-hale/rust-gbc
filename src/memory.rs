use std::sync::{Arc, Mutex};

pub struct Memory {
    m: [u8; 0xFFFF],
}

impl Memory {
    pub fn new() -> Memory {
        Memory { m: [0u8; 0xFFFF] }
    }

    pub fn arc() -> Arc<Mutex<Memory>> {
        Arc::new(Mutex::new(Memory::new()))
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
        self.m[addr as usize] = data;
    }

    pub fn write_word(&mut self, addr: u16, data: u16) {
        let low = (data & 0x00FF) as u8;
        let high = ((data & 0xFF00) >> 8) as u8;
        self.write(addr, low);
        self.write(addr + 1, high);
    }

    pub fn inc(&mut self, addr: u16) -> u8 {
        self.m[addr as usize] += 1;
        self.read(addr)
    }

    pub fn dec(&mut self, addr: u16) -> u8 {
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

    fn dec() {
        let mut mem = Memory::new();
        let addr = 0xFFEE;
        let byte = 0xDE;
        mem.m[addr as usize] = byte;
        mem.dec(addr);
        assert_eq!(mem.read(addr), byte - 1);
    }
}
