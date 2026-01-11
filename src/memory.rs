pub struct Memory {
    m: [u8; 0xFFFF],
}

impl Memory {
    pub fn new() -> Memory {
        Memory { m: [0u8; 0xFFFF] }
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
}

#[cfg(test)]
mod test {
    use super::*;

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
}
