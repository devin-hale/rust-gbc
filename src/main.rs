#![allow(dead_code)]

use sdl2::{
    event::{Event, WindowEvent},
    keyboard::Keycode,
    pixels::{Color, PixelFormatEnum},
};

use crate::{cpu::CPU, memory::Memory, ppu::PPU};

mod cpu;
mod memory;
mod ppu;
mod utils;

const WINDOW_W: u32 = 320;
const WINDOW_H: u32 = 288;

const GB_LCD_W: u32 = 160;
const GB_LCD_H: u32 = 144;

const BG_TILE_WIDTH: u32 = 32 * 8;
const BG_TILE_HEIGHT: u32 = 32 * 8;

const PIXEL_W: u32 = WINDOW_W / GB_LCD_W;
const PIXEL_H: u32 = WINDOW_H / GB_LCD_H;

fn main() {
    let ctx = sdl2::init().unwrap();
    let vs = ctx.video().unwrap();

    let window = vs
        .window("rust-gbc", BG_TILE_WIDTH, BG_TILE_HEIGHT)
        .position_centered()
        .resizable()
        .opengl()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    let mut event_pump = ctx.event_pump().unwrap();
    let mut running = true;

    let mem = Memory::arc();
    let _cpu = CPU::new(&mem);
    let mut ppu = PPU::new(&mem);

    while running {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    running = false;
                }
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::Resized(w, h) => {
                        canvas.window_mut().set_size(w as u32, h as u32).unwrap();
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        canvas.set_draw_color(Color::BLACK);
        canvas.clear();

        let tc = canvas.texture_creator();
        let mut t = tc
            .create_texture_streaming(PixelFormatEnum::RGB24, BG_TILE_WIDTH, BG_TILE_HEIGHT)
            .unwrap();
        t.with_lock(None, |buf: &mut [u8], pitch: usize| {
            ppu.draw_bg_tiles(buf, pitch);
        })
        .unwrap();
        canvas.copy(&t, None, None).unwrap();

        canvas.present();
    }
}
