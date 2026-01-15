#![allow(dead_code)]

mod cpu;
mod memory;
mod utils;

use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use cpu::CPU;
use memory::Memory;

use crate::cpu::T_CYCLE_PERIOD;

fn main() {
    let mem = Arc::new(Mutex::new(Memory::new()));
    #[allow(unused_variables)]
    let mut cpu = CPU::new(&mem);

    let ticks = Arc::new(Mutex::new(0u128));
    let ticks_clone = Arc::clone(&ticks);
    thread::spawn(move || {
        let start = Instant::now();
        let tikref = ticks_clone;
        thread::sleep(Duration::from_secs(1));
        loop {
            let ticks = *tikref.lock().unwrap();
            println!("ticks/s: {}", ticks/start.elapsed().as_secs() as u128);
            thread::sleep(Duration::from_secs(1));
        }
    });

    let t_cycle = Duration::from_nanos(T_CYCLE_PERIOD as u64);
    let mut tick: Option<Instant> = None;
    let mut cycles = 0u8;
    loop {
        match tick {
            Some(t) => {
                if t.elapsed().as_nanos() >= (cycles as u128) * t_cycle.as_nanos() {
                    tick = Some(Instant::now());
                    *ticks.lock().unwrap() += 4;
                    cycles = cpu.progress().unwrap();
                } else {
                    thread::sleep(t_cycle);
                }
            }
            None => {
                tick = Some(Instant::now());
                cycles = cpu.progress().unwrap();
            }
        }
    }
}
