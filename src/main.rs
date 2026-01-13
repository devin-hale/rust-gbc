#![allow(dead_code)]

mod cpu;
mod memory;
mod utils;

use std::sync::{Arc, Mutex};

use cpu::CPU;
use memory::Memory;

fn main() {
    let mem = Arc::new(Mutex::new(Memory::new()));
    #[allow(unused_variables)]
    let cpu = CPU::new(&mem);
}
