use std::sync::{Arc, Mutex, MutexGuard};

use crate::{memory::Memory, utils::bit};

pub struct PPU {
    mem: Arc<Mutex<Memory>>,
}

enum TileArea {
    Low,
    High,
}

enum Color {
    White,
    LightGrey,
    DarkGrey,
    Black,
}

impl From<u8> for Color {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::White,
            1 => Self::LightGrey,
            2 => Self::DarkGrey,
            3 => Self::Black,
            _ => panic!("invalid color value"),
        }
    }
}

struct Palette(u8);

impl Palette {
    fn id(&self, h: u8, l: u8) -> Color {
        let high = bit::get(self.0, h);
        let low = bit::get(self.0, l);
        ((high << 1) | low).into()
    }
    fn id_3(&self) -> Color {
        self.id(7, 6)
    }
    fn id_2(&self) -> Color {
        self.id(5, 4)
    }
    fn id_1(&self) -> Color {
        self.id(3, 2)
    }
    fn id_0(&self) -> Color {
        self.id(1, 0)
    }
}

impl From<u8> for Palette {
    fn from(value: u8) -> Self {
        Palette(value)
    }
}

impl PPU {
    fn mem<'m>(&'m mut self) -> MutexGuard<'m, Memory> {
        self.mem.lock().expect("error acquiring Memory mutex lock")
    }

    fn lcd_ppu_en(&mut self) -> bool {
        bit::is_set(self.mem().lcdc(), 7)
    }

    fn window_tile_map(&mut self) -> TileArea {
        if bit::is_set(self.mem().lcdc(), 6) {
            return TileArea::High;
        }
        TileArea::Low
    }

    fn window_enable(&mut self) -> bool {
        bit::is_set(self.mem().lcdc(), 5)
    }

    fn background_area(&mut self) -> TileArea {
        if bit::is_set(self.mem().lcdc(), 4) {
            return TileArea::High;
        }
        TileArea::Low
    }

    fn background_tile_map(&mut self) -> TileArea {
        if bit::is_set(self.mem().lcdc(), 3) {
            return TileArea::High;
        }
        TileArea::Low
    }

    // 0 == 8x8
    // 1 == 8x16
    fn obj_size(&mut self) -> bool {
        bit::is_set(self.mem().lcdc(), 2)
    }

    fn obj_enable(&mut self) -> bool {
        bit::is_set(self.mem().lcdc(), 1)
    }

    fn bg_window_enable(&mut self) -> bool {
        bit::is_set(self.mem().lcdc(), 0)
    }

    fn bg_palette(&mut self) -> Palette {
        self.mem().bgp().into()
    }

    fn obj_palette_0(&mut self) -> Palette {
        self.mem().obj_palette_0().into()
    }

    fn obj_palette_1(&mut self) -> Palette {
        self.mem().obj_palette_1().into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile([u8; 16]);

impl Tile {
    pub fn new() -> Tile {
        Tile([0u8; 16])
    }

    pub fn line(&self, n: usize) -> &[u8] {
        &self.0[(n * 2)..(n * 2 + 1)]
    }

    pub fn line_as_ci(&self, n: usize) -> Vec<u8> {
        let a = self.0[n * 2];
        let b = self.0[n * 2 + 1];
        let mut row = vec![];
        for i in 7u8..0 {
            let low = bit::get(b, i);
            let high = bit::get(a, i);
            row.push((high << 1) | low);
        }
        row
    }
}

impl From<&[u8]> for Tile {
    fn from(value: &[u8]) -> Self {
        assert_eq!(value.len(), 16);
        let mut t = Tile::new();
        for (i, b) in value.iter().enumerate() {
            t.0[i] = *b
        }
        t
    }
}

impl From<[u8; 16]> for Tile {
    fn from(value: [u8; 16]) -> Self {
        assert_eq!(value.len(), 16);
        let mut t = Tile::new();
        t.0 = value.clone();
        t
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn to_tile() {
        let t_arr: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let t1: Tile = t_arr.clone().into();
        let t2: Tile = (&t_arr[..]).into();
        assert_eq!(t1, t2);
        let l1 = t1.line(0);
        let l2 = t2.line(0);
        assert_eq!(l1, l2);
    }
}
