pub fn get(byte: u8, n: u8) -> u8 {
    (byte >> n) & 1
}

pub fn is_set(byte: u8, n: u8) -> bool {
    get(byte, n) == 1
}

pub fn get_u16(word: u16, n: u8) -> u16 {
    (word >> n) & 1
}

pub fn set(byte: &mut u8, n: u8) {
    *byte = *byte | (1 << n)
}

pub fn reset(byte: &mut u8, n: u8) {
    *byte = *byte & !(1 << n)
}

pub fn toggle(byte: &mut u8, n: u8) {
    *byte = *byte ^ (1 << n)
}

fn fill_to(n: u8) -> u8 {
    let mut byte = 0;
    for _ in 0u8..=n {
        byte |= 0b1;
        byte = byte << 1;
    }
    byte
}

pub fn sub_borrow(a: u8, b: u8, n: u8) -> bool {
    let fill = fill_to(n);
    let a = a & fill;
    let b = b & fill;
    let r = a - b;
    is_set(a, n) && !is_set(r, n)
}

pub fn add_overflow(a: u8, b: u8, n: u8) -> bool {
    let bit = 1 << n;
    let a = a & bit;
    let b = b & bit;
    get(a + b, n + 1) == 1
}

pub fn add_overflow_u16(a: u16, b: u16, n: u8) -> bool {
    let bit = 1 << n;
    let a = a & bit;
    let b = b & bit;
    get_u16(a + b, n + 1) == 1
}

#[cfg(test)]
mod tests {
    use crate::bit;

    #[test]
    fn get() {
        let byte = 0b0101_0101;
        assert_eq!(bit::get(byte, 0), 1);
        assert_eq!(bit::get(byte, 1), 0);
        assert_eq!(bit::get(byte, 2), 1);
        assert_eq!(bit::get(byte, 3), 0);
        assert_eq!(bit::get(byte, 4), 1);
        assert_eq!(bit::get(byte, 5), 0);
        assert_eq!(bit::get(byte, 6), 1);
        assert_eq!(bit::get(byte, 7), 0);
    }

    #[test]
    fn set() {
        let mut byte = 0b0101_0101;
        bit::set(&mut byte, 1);
        assert_eq!(bit::get(byte, 1), 1);
    }

    #[test]
    fn reset() {
        let mut byte = 0b0101_0101;
        bit::reset(&mut byte, 1);
        assert_eq!(bit::get(byte, 1), 0);
    }

    #[test]
    fn toggle() {
        let mut byte = 0b0101_0101;
        bit::toggle(&mut byte, 1);
        assert_eq!(bit::get(byte, 1), 1);
        bit::toggle(&mut byte, 1);
        assert_eq!(bit::get(byte, 1), 0);
    }

    #[test]
    fn add_overflow() {
        let a = 0b0001_0000;
        let b = 0b0001_0000;
        assert!(bit::add_overflow(a, b, 4));
        assert!(!bit::add_overflow(a, 0, 4));
    }

    #[test]
    fn add_overflow_u16() {
        let a = 0b0001_0000 as u16;
        let b = 0b0001_0000 as u16;
        assert!(bit::add_overflow_u16(a, b, 4));
        assert!(!bit::add_overflow_u16(a, 0, 4));
    }

    #[test]
    fn fill_to() {
        assert_ne!(bit::fill_to(4), 0b0001_1111);
    }

    #[test]
    fn sub_borrow() {
        let a = 0b0001_0000;
        let b = 0b0000_1000;
        assert!(bit::sub_borrow(a, b, 4));
        assert!(!bit::sub_borrow(a, 0, 4));
    }
}
