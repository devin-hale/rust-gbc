use std::sync::{Arc, Mutex, MutexGuard};

use crate::{memory::Memory, utils::bit};

pub const VRAM_START: usize = 0x8000;
pub const VRAM_BLOCK_0_START: usize = VRAM_START;
pub const VRAM_BLOCK_0_END: usize = 0x87FF;
pub const VRAM_BLOCK_1_START: usize = 0x8800;
pub const VRAM_BLOCK_1_END: usize = 0x8FFF;
pub const VRAM_BLOCK_2_START: usize = 0x9000;
pub const VRAM_BLOCK_2_END: usize = 0x97FF;
pub const VRAM_END: usize = 0x9FFF;
pub const VRAM_SIZE: usize = VRAM_END - VRAM_START + 1;

pub const BG_TILE_DATA_0_START: usize = 0x8800;
pub const BG_TILE_DATA_0_END: usize = 0x97FF;
pub const BG_TILE_DATA_1_START: usize = VRAM_BLOCK_0_START;
pub const BG_TILE_DATA_1_END: usize = 0x8FFF;
pub const BG_TILE_DATA_SIZE: usize = BG_TILE_DATA_1_END - BG_TILE_DATA_1_START + 1;

pub const BG_TILE_MAP_0_START: usize = 0x9800;
pub const BG_TILE_MAP_0_END: usize = 0x9BFF;
pub const BG_TILE_MAP_1_START: usize = 0x9C00;
pub const BG_TILE_MAP_1_END: usize = 0x9FFF;
pub const BG_TILE_MAP_SIZE: usize = BG_TILE_MAP_0_END - BG_TILE_MAP_0_START + 1;

pub const OBJ_TILE_DATA_START: usize = 0x8000;
pub const OBJ_TILE_DATA_END: usize = 0x8FFF;
pub const OBJ_TILE_DATA_LEN: usize = OBJ_TILE_DATA_END - OBJ_TILE_DATA_START;

pub const SCY: usize = 0xFF42;
pub const SCX: usize = 0xFF43;

pub struct PPU {
    mem: Arc<Mutex<Memory>>,
}

impl PPU {
    fn mem<'m>(&'m mut self) -> MutexGuard<'m, Memory> {
        self.mem.lock().expect("error acquiring Memory mutex lock")
    }

    fn lcd_ppu_en(&mut self) -> bool {
        bit::is_set(self.mem().lcdc(), 7)
    }

    //fn window_tile_map(&mut self) -> TileArea {
    //    if bit::is_set(self.mem().lcdc(), 6) {
    //        return TileArea::High;
    //    }
    //    TileArea::Low
    //}

    //fn window_enable(&mut self) -> bool {
    //    bit::is_set(self.mem().lcdc(), 5)
    //}

    //fn background_area(&mut self) -> TileArea {
    //    if bit::is_set(self.mem().lcdc(), 4) {
    //        return TileArea::High;
    //    }
    //    TileArea::Low
    //}

    fn bg_tile_data(&mut self) -> Vec<Tile> {
        let bg_mode = bit::is_set(self.mem().lcdc(), 4);
        let mem = self.mem();
        let td_raw;
        if bg_mode {
            td_raw = &mem[BG_TILE_DATA_1_START..BG_TILE_DATA_1_END];
        } else {
            td_raw = &mem[BG_TILE_DATA_0_START..BG_TILE_DATA_0_END];
        }
        assert_eq!(td_raw.len(), BG_TILE_DATA_SIZE);
        let mut td: Vec<Tile> = vec![];
        for i in 0..(BG_TILE_DATA_SIZE / 16) {
            let offset = i * 16;
            let tile: Tile = (&td_raw[offset..(offset + 16)]).into();
            td.push(tile);
        }
        assert_eq!(td.len(), BG_TILE_DATA_SIZE / 16);
        td
    }

    fn bg_tile_map(&mut self) -> [u8; BG_TILE_MAP_SIZE] {
        let bg_mode = bit::is_set(self.mem().lcdc(), 3);
        let mem = self.mem();
        let mut tm_raw = [0u8; BG_TILE_MAP_SIZE];
        if bg_mode {
            tm_raw.copy_from_slice(&mem[BG_TILE_MAP_1_START..BG_TILE_MAP_1_END]);
        } else {
            tm_raw.copy_from_slice(&mem[BG_TILE_MAP_0_START..BG_TILE_MAP_0_END]);
        }
        tm_raw
    }

    fn bg_viewport(&mut self) -> (u8, u8) {
        let sy = self.mem()[SCY].wrapping_add(143);
        let sx = self.mem()[SCX].wrapping_add(159);
        (sy, sx)
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
        self.mem().bg_pal().into()
    }

    fn obj_palette_0(&mut self) -> Palette {
        self.mem().obj_pal_0().into()
    }

    fn obj_palette_1(&mut self) -> Palette {
        self.mem().obj_pal_1().into()
    }

    fn get_fb(&mut self) {
        let td = self.bg_tile_data();
        let tmap = self.bg_tile_map();

        let (b, r) = self.bg_viewport();

        for t in tmap {
            
        }

        //fb
    }
}

pub enum Color {
    White,
    LightGrey,
    DarkGrey,
    Black,
}

impl Color {
    fn to_rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::White => (255, 255, 255),
            Self::LightGrey => (166, 166, 166),
            Self::DarkGrey => (83, 83, 83),
            Self::Black => (0, 0, 0),
        }
    }
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

enum PPUMode {
    OAM,
    Draw,
    HBlank,
    VBlank,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile([u8; 16]);

impl Tile {
    pub fn new() -> Tile {
        Tile([0u8; 16])
    }

    pub fn line(&self, n: usize) -> &[u8] {
        &self.0[(n * 2)..(n * 2 + 1)]
    }

    pub fn line_as_ci(&self, n: usize) -> Vec<Color> {
        let a = self.0[n * 2];
        let b = self.0[n * 2 + 1];
        let mut row: Vec<Color> = vec![];
        for i in 7u8..0 {
            let low = bit::get(b, i);
            let high = bit::get(a, i);
            row.push(((high << 1) | low).into());
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
