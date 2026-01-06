#![allow(dead_code)]

mod bit;
mod registers;
mod cpu;
mod instructions;

fn main() {
    println!("{}", instructions::R8::HlMem);
}
