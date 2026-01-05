pub struct Register {
    data: u16,
}

pub enum Level {
    Low,
    High,
}

impl Register {
    pub fn new() -> Register {
        Register { data: 0 }
    }

    pub fn from_word(data: u16) -> Register {
        Register { data }
    }

    pub fn data(&self) -> u16 {
        self.data
    }

    pub fn low(&self) -> u8 {
        (self.data & 0x00FF) as u8
    }

    pub fn high(&self) -> u8 {
        ((self.data & 0xFF00) >> 8) as u8
    }

    pub fn write_byte(&mut self, level: Level, data: u8) {
        match level {
            Level::Low => self.data = (self.data & 0xFF00) | data as u16,
            Level::High => self.data = (self.data & 0x00FF) | ((data as u16) << 8),
        }
    }

    pub fn write_word(&mut self, data: u16) {
        self.data = data
    }
}

pub struct ProgramCounter {
    data: u16,
}

impl ProgramCounter {
    pub fn new() -> ProgramCounter {
        ProgramCounter { data: 0 }
    }

    pub fn current(&self) -> u16 {
        self.data
    }

    pub fn inc(&mut self) {
        self.data += 1;
    }

    pub fn set(&mut self, data: u16) {
        self.data = data;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_word() {
        let r = Register::new();
        assert_eq!(0, r.data());

        let word = 0xFE;
        let r = Register::from_word(word);
        assert_eq!(word, r.data());
    }

    #[test]
    fn write_word() {
        let word = 0xFE;
        let mut r = Register::from_word(word);

        let word = 0xBB;
        r.write_word(word);
        assert_eq!(word, r.data());
    }

    #[test]
    fn read_byte() {
        let word: u16 = 0xFEFE;
        let low = (word & 0x00FF) as u8;
        let high = ((word & 0xFF00) >> 8) as u8;

        let r = Register::from_word(word);
        assert_eq!(low, r.low());
        assert_eq!(high, r.high());
    }

    #[test]
    fn write_byte_low() {
        let word = 0xFEFE;
        let mut r = Register::from_word(word);

        let byte = 0xBB;
        r.write_byte(Level::Low, byte);

        let expected_word = 0xFEBB;

        assert_eq!(expected_word, r.data());
        assert_eq!(byte, r.low());
    }

    #[test]
    fn write_byte_high() {
        let word: u16 = 0xFEFE;
        let mut r = Register::from_word(word);

        let byte = 0xBB;
        r.write_byte(Level::High, byte);

        let expected_word = 0xBBFE;

        assert_eq!(expected_word, r.data());
        assert_eq!(byte, r.high());
    }
}
