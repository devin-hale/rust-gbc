pub struct Register {
    label: Option<String>,
    data: u16,
}

#[derive(Clone, Copy)]
pub enum Level {
    Low,
    High,
}

impl Register {
    pub fn new() -> Register {
        Register {
            data: 0,
            label: None,
        }
    }

    pub fn from_word(data: u16) -> Register {
        Register { data, label: None }
    }

    pub fn set_label(&mut self, l: String) {
        self.label = Some(l)
    }

    pub fn label(&self) -> Option<String> {
        self.label.clone()
    }

    pub fn val(&self) -> u16 {
        self.data
    }

    pub fn low(&self) -> u8 {
        (self.data & 0x00FF) as u8
    }

    pub fn high(&self) -> u8 {
        ((self.data & 0xFF00) >> 8) as u8
    }

    pub fn read_byte(&self, level: Level) -> u8 {
        match level {
            Level::Low => self.low(),
            Level::High => self.high(),
        }
    }

    pub fn write_byte(&mut self, level: Level, data: u8) {
        match level {
            Level::Low => self.data = (self.data & 0xFF00) | data as u16,
            Level::High => self.data = (self.data & 0x00FF) | ((data as u16) << 8),
        }
    }

    pub fn write(&mut self, data: u16) {
        self.data = data
    }

    pub fn inc(&mut self) {
        self.data += 1
    }

    pub fn dec(&mut self) {
        self.data -= 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_word() {
        let r = Register::new();
        assert_eq!(0, r.val());

        let word = 0xFE;
        let r = Register::from_word(word);
        assert_eq!(word, r.val());
    }

    #[test]
    fn write_word() {
        let word = 0xFE;
        let mut r = Register::from_word(word);

        let word = 0xBB;
        r.write(word);
        assert_eq!(word, r.val());
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

        assert_eq!(expected_word, r.val());
        assert_eq!(byte, r.low());
    }

    #[test]
    fn write_byte_high() {
        let word: u16 = 0xFEFE;
        let mut r = Register::from_word(word);

        let byte = 0xBB;
        r.write_byte(Level::High, byte);

        let expected_word = 0xBBFE;

        assert_eq!(expected_word, r.val());
        assert_eq!(byte, r.high());
    }
}
