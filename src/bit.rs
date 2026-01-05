fn get(byte: u8, n: u8) -> u8 {
    (byte >> n) & 1
}

fn set(byte: &mut u8, n: u8) {
    *byte = *byte | (1 << n)
}

fn reset(byte: &mut u8, n: u8) {
    *byte = *byte & !(1 << n)
}

fn toggle(byte: &mut u8, n: u8) {
    *byte = *byte ^ (1 << n)
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
}
