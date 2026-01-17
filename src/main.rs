#![allow(dead_code)]

use sdl2::{event::Event, keyboard::Keycode, pixels::Color};

mod cpu;
mod ppu;
mod memory;
mod utils;

fn main() {
    let ctx = sdl2::init().unwrap();
    let vs = ctx.video().unwrap();

    let window = vs
        .window("rust-gbc", 300, 300)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    let mut event_pump = ctx.event_pump().unwrap();
    let mut running = true;

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
                _ => {}
            }
        }

        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        canvas.present();
    }
}
