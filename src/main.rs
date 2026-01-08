#![allow(dead_code)]

use crate::instructions::Instruction;

mod bit;
mod cpu;
mod instructions;
mod memory;
mod registers;

fn main() {
    let i = Instruction::nop();
    println!("{}", i);
}
