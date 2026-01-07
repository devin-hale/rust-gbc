#![allow(dead_code)]

mod bit;
mod cpu;
mod instructions;
mod memory;
mod registers;

fn main() {
    println!("{}", instructions::R8::HlMem);
}
