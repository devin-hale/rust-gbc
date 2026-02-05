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

const GB_LCD_W: usize = 160;
const GB_LCD_H: usize = 144;

pub struct PPU {
    mem: Arc<Mutex<Memory>>,
}

impl PPU {
    pub fn new(mem: &Arc<Mutex<Memory>>) -> PPU {
        PPU {
            mem: Arc::clone(mem),
        }
    }
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

    fn background(&mut self) -> Background {
        let mem = self.mem();

        let bg_data_mode = bit::is_set(mem.lcdc(), 4);
        let mut td = [0u8; BG_TILE_DATA_SIZE];
        if bg_data_mode {
            td.copy_from_slice(&mem[BG_TILE_DATA_1_START..=BG_TILE_DATA_1_END]);
        } else {
            td.copy_from_slice(&mem[BG_TILE_DATA_0_START..=BG_TILE_DATA_0_END]);
        }

        let bg_tile_mode = bit::is_set(mem.lcdc(), 3);
        let mut tm = [0u8; BG_TILE_MAP_SIZE];
        if bg_tile_mode {
            tm.copy_from_slice(&mem[BG_TILE_MAP_1_START..=BG_TILE_MAP_1_END]);
        } else {
            tm.copy_from_slice(&mem[BG_TILE_MAP_0_START..=BG_TILE_MAP_0_END]);
        }
        Background { map: tm, data: td }
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

    pub fn draw_bg_tiles(&mut self, buf: &mut [u8], pitch: usize) {
        let bg = self.background();
        bg.draw(buf, pitch);
    }
}

#[derive(Debug, PartialEq)]
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
pub struct Tile {
    buf: [u8; 16],
}

impl Tile {
    pub const LINE_SIZE_RGB24: usize = 24;
    pub const TILE_SIZE_RGB24: usize = Self::LINE_SIZE_RGB24 * 8;

    pub fn new() -> Tile {
        Tile { buf: [0u8; 16] }
    }

    fn line(&self, n: usize) -> &[u8] {
        &self.buf[(n * 2)..(n * 2 + 1)]
    }

    pub fn pixel(&self, x: usize, y: usize) -> Color {
        let a = self.buf[y * 2];
        let b = self.buf[y * 2 + 1];
        let low = bit::get(b, x as u8);
        let high = bit::get(a, x as u8);
        ((high << 1) | low).into()
    }

    pub fn line_as_ci(&self, n: usize) -> Vec<Color> {
        let a = self.buf[n * 2];
        let b = self.buf[n * 2 + 1];
        let mut row: Vec<Color> = vec![];
        for i in 7u8..0 {
            let low = bit::get(b, i);
            let high = bit::get(a, i);
            row.push(((high << 1) | low).into());
        }
        row
    }

    pub fn line_rgb24(&self, n: usize) -> [u8; Self::LINE_SIZE_RGB24] {
        let a = self.buf[n * 2];
        let b = self.buf[n * 2 + 1];
        let mut colors: Vec<(u8, u8, u8)> = vec![];
        let mut line = [0u8; Self::LINE_SIZE_RGB24];
        for i in 7u8..0 {
            let low = bit::get(b, i);
            let high = bit::get(a, i);
            let color: Color = ((high << 1) | low).into();
            colors.push(color.to_rgb());
        }
        for i in 0..8u8 as usize {
            let offset = 3 * i;
            let c = colors[i];
            line[offset] = c.0;
            line[offset + 1] = c.1;
            line[offset + 2] = c.2;
        }
        line
    }
}

impl From<&[u8]> for Tile {
    fn from(value: &[u8]) -> Self {
        assert_eq!(value.len(), 16);
        let mut t = Tile::new();
        for (i, b) in value.iter().enumerate() {
            t.buf[i] = *b
        }
        t
    }
}

impl From<[u8; 16]> for Tile {
    fn from(value: [u8; 16]) -> Self {
        assert_eq!(value.len(), 16);
        let mut t = Tile::new();
        t.buf = value.clone();
        t
    }
}

struct Background {
    map: [u8; BG_TILE_MAP_SIZE],
    data: [u8; BG_TILE_DATA_SIZE],
}

impl Background {
    fn draw(&self, buf: &mut [u8], pitch: usize) {
        for (i, ti) in self.map.iter().enumerate() {
            let tile = self.tile_from_index(*ti);
            let pixel_row = (i / 32) * 8;
            let pixel_col = (i % 32) * 8;

            for y in 0..8 as usize {
                let offset_y = pixel_row + y;
                for x in 0..8 as usize {
                    let color = tile.pixel(x, y);
                    let rgb = color.to_rgb();
                    let offset_x = pixel_col + x;
                    let offset = (offset_y * pitch) + offset_x * 3;

                    buf[offset] = rgb.0;
                    buf[offset + 1] = rgb.1;
                    buf[offset + 2] = rgb.2;
                }
            }
        }
    }

    fn tile_from_index(&self, i: u8) -> Tile {
        let offset = i * 16;
        let tile_arr = &self.data[(offset as usize)..(offset as usize + 16)];
        tile_arr.into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const TEST_TILE: [u8; 16] = [
        0x3C, 0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x7E, 0x5E, 0x7E, 0x0A, 0x7C, 0x56, 0x38,
        0x7C,
    ];

    #[test]
    fn to_tile() {
        let t1: Tile = TEST_TILE.clone().into();
        let t2: Tile = (&TEST_TILE[..]).into();
        assert_eq!(t1, t2);

        let l1 = t1.line(0);
        let l2 = t2.line(0);
        assert_eq!(l1, l2);

        let pixel = t1.pixel(0, 0);
        assert_eq!(pixel, Color::White);
    }
}
