#![allow(dead_code)]

use std::{
    collections::HashMap,
    fs,
    sync::{Arc, Mutex},
    time::Instant,
};

use sdl2::{
    VideoSubsystem,
    event::{Event, WindowEvent},
    keyboard::Keycode,
    pixels::{Color, PixelFormatEnum},
    render::Canvas,
};

use crate::{
    cpu::{CPU, T_CYCLE_PRD_NS},
    memory::Memory,
    ppu::PPU,
};

mod cpu;
mod memory;
mod ppu;
mod utils;

const BG_TILE_WIDTH: u32 = 32 * 8;
const BG_TILE_HEIGHT: u32 = 32 * 8;

const WINDOW_W: u32 = 320;
const WINDOW_H: u32 = 288;

const GB_LCD_W: u32 = 160;
const GB_LCD_H: u32 = 144;

const PIXEL_W: u32 = WINDOW_W / GB_LCD_W;
const PIXEL_H: u32 = WINDOW_H / GB_LCD_H;

enum WindowType {
    BackgroundTiles,
    Screen,
}

impl WindowType {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::BackgroundTiles => (BG_TILE_WIDTH, BG_TILE_HEIGHT),
            Self::Screen => (GB_LCD_W, GB_LCD_H),
        }
    }
}

struct WindowOptions {
    pub w_type: WindowType,
    pub width: u32,
    pub height: u32,
    pub name: &'static str,
}

struct Window {
    ppu: Arc<Mutex<PPU>>,
    canvas: Canvas<sdl2::video::Window>,
    active: bool,
    w_type: WindowType,
    base_dimensions: (u32, u32),
}

impl Window {
    fn id(&self) -> u32 {
        self.canvas.window().id()
    }

    pub fn draw(&mut self) {
        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();

        let tc = self.canvas.texture_creator();
        let mut t = tc
            .create_texture_streaming(
                PixelFormatEnum::RGB24,
                self.base_dimensions.0,
                self.base_dimensions.1,
            )
            .unwrap();
        t.with_lock(None, |buf: &mut [u8], pitch: usize| {
            self.draw_by_type(buf, pitch);
        })
        .unwrap();
        self.canvas.copy(&t, None, None).unwrap();
        self.canvas.present();
    }

    fn draw_by_type(&mut self, buf: &mut [u8], pitch: usize) {
        let mut ppu = self.ppu.lock().unwrap();
        match self.w_type {
            WindowType::BackgroundTiles => ppu.draw_bg_tiles(buf, pitch),
            WindowType::Screen => ppu.draw_screen(buf, pitch),
        }
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        self.canvas
            .window_mut()
            .set_size(w as u32, h as u32)
            .unwrap();
    }
}

struct WindowManager<'w> {
    vs: &'w VideoSubsystem,
    wm: HashMap<u32, Arc<Mutex<Window>>>,
    ppu: Arc<Mutex<PPU>>,
}

impl<'w> WindowManager<'w> {
    fn new(vs: &'w VideoSubsystem, ppu: &Arc<Mutex<PPU>>) -> WindowManager<'w> {
        WindowManager {
            vs,
            ppu: Arc::clone(ppu),
            wm: HashMap::new(),
        }
    }

    pub fn create_window(&mut self, opts: WindowOptions) -> Arc<Mutex<Window>> {
        let win = self
            .vs
            .window(opts.name, opts.width, opts.height)
            .position_centered()
            .resizable()
            .opengl()
            .build()
            .unwrap();
        let canvas = win.into_canvas().build().unwrap();
        let w = Window {
            ppu: Arc::clone(&self.ppu),
            canvas,
            active: true,
            w_type: opts.w_type,
            base_dimensions: (opts.width, opts.height),
        };
        let id = w.id();
        let w = Arc::new(Mutex::new(w));
        self.wm.insert(id, Arc::clone(&w));
        Arc::clone(&w)
    }

    pub fn get(&self, id: u32) -> Option<&Arc<Mutex<Window>>> {
        self.wm.get(&id)
    }

    pub fn close(&self, id: u32) {
        match self.get(id) {
            Some(_) => {
                //let win = wa.lock().unwrap();
            }
            _ => {}
        }
    }

    pub fn bg_tile_viewer(&mut self) -> Arc<Mutex<Window>> {
        self.create_window(WindowOptions {
            w_type: WindowType::BackgroundTiles,
            width: BG_TILE_WIDTH,
            height: BG_TILE_HEIGHT,
            name: "background tiles",
        })
    }

    pub fn screen(&mut self) -> Arc<Mutex<Window>> {
        self.create_window(WindowOptions {
            w_type: WindowType::Screen,
            width: GB_LCD_W,
            height: GB_LCD_H,
            name: "screen",
        })
    }
}

fn main() {
    let rom = fs::read("pkm_red.gb").unwrap();

    let ctx = sdl2::init().unwrap();
    let vs = ctx.video().unwrap();

    let mem = Memory::arc();
    mem.lock().unwrap().init();
    mem.lock().unwrap().load_rom(&rom);
    let mut cpu = CPU::new(&mem);
    let ppu = Arc::new(Mutex::new(PPU::new(&mem)));

    let mut wm = WindowManager::new(&vs, &ppu);

    let screen = wm.screen();
    let bg_tile_viewer = wm.bg_tile_viewer();

    let mut event_pump = ctx.event_pump().unwrap();

    let mut running = true;
    while running {
        let now = Instant::now();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    running = false;
                    println!("NO RUN");
                }
                Event::Window {
                    win_event,
                    window_id,
                    ..
                } => match win_event {
                    WindowEvent::Close => {}
                    WindowEvent::Resized(w, h) => {
                        let odw = wm.get(window_id);
                        match odw {
                            Some(dw) => {
                                dw.lock().unwrap().resize(w, h);
                            }
                            None => {}
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        bg_tile_viewer.lock().unwrap().draw();
        screen.lock().unwrap().draw();
        let cycles_to_adv = now.elapsed().as_nanos() / T_CYCLE_PRD_NS as u128;
        let mut cycles = 0;
        while cycles < cycles_to_adv {
            cycles += cpu.tick().unwrap() as u128;
        }
    }
}
