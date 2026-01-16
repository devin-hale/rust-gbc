#![allow(dead_code)]

mod cpu;
mod memory;
mod utils;

use std::{
    thread,
    time::{Duration, Instant},
};

use cpu::CPU;
use memory::Memory;

use crate::cpu::T_CYCLE_PERIOD;

fn main() {
    let mem = Memory::arc();
    let mut cpu = CPU::new(&mem);

    let start = Instant::now();
    let t_cycle = Duration::from_nanos(T_CYCLE_PERIOD as u64);
    let mut total_cycles = 0u128;
    loop {
        let cycles = cpu.tick().unwrap();
        total_cycles += cycles as u128;
        if total_cycles >= 50_000_000 {
            break;
        }
        thread::sleep(t_cycle * cycles as u32);
    }

    let el = start.elapsed();
    println!("time elapsed: {:?}", el);
    println!("avg: {} cycles/s", total_cycles / el.as_secs() as u128);
}
