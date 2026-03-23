use std::fmt::Display;

use thiserror::Error;

use crate::utils::bit;

#[derive(Debug, Error)]
pub enum Error {
    #[error("'0x{0:x}' is not a valid R8 value")]
    InvalidR8(u8),

    #[error("'0x{0:x}' is not a valid R16 value")]
    InvalidR16(u8),

    #[error("'0x{0:x}' is not a valid R16Stk value")]
    InvalidR16Stk(u8),

    #[error("'0x{0:x}' is not a valid R16Mem value")]
    InvalidR16Mem(u8),

    #[error("'0x{0:x}' is not a valid Cond value")]
    InvalidCond(u8),

    #[error("invalid opcode: 0x{0:x}")]
    InvalidOpCode(u8),

    #[error("opcode '0x{0:x}' is unimplemented")]
    Unimplemented(u8),

    #[error("unknown")]
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    opcode: u8,
    len: u8,
    cycles: (u8, u8),

    // for purposes for proper string representation when executing
    n8: Option<u8>,
    n16_lo: Option<u8>,
    n16_hi: Option<u8>,
    e: Option<i8>,

    steps: Vec<Step>,
    eager: bool,
    branched: bool,
    done: bool,
}

impl Instruction {
    pub fn new() -> Instruction {
        Instruction::default()
    }

    pub fn cycles(&self) -> u8 {
        match self.branched {
            false => self.cycles.0,
            true => self.cycles.1,
        }
    }

    pub fn branch_cycles(&self) -> u8 {
        self.cycles.1
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    pub fn steps_mut(&mut self) -> &mut [Step] {
        &mut self.steps
    }

    pub fn done(&self) -> bool {
        self.done
    }

    pub fn eager(&self) -> bool {
        self.eager
    }

    pub fn complete(&mut self) {
        self.done = true
    }

    pub fn set_n8(&mut self, n: u8) {
        self.n8 = Some(n)
    }

    pub fn set_n16_lo(&mut self, n: u8) {
        self.n16_lo = Some(n)
    }

    pub fn set_n16_hi(&mut self, n: u8) {
        self.n16_hi = Some(n)
    }

    pub fn n16(&self) -> u16 {
        let low = self.n16_lo.unwrap() as u16;
        let high = self.n16_hi.unwrap() as u16;
        (high << 8) | low
    }

    pub fn n16_lo(&self) -> u8 {
        self.n16_lo.unwrap()
    }

    pub fn n16_hi(&self) -> u8 {
        self.n16_hi.unwrap()
    }

    pub fn n8(&self) -> u8 {
        self.n8.unwrap()
    }

    pub fn e(&self) -> i8 {
        self.e.unwrap()
    }

    pub fn set_e(&mut self, e: i8) {
        self.e = Some(e)
    }
}

impl Instruction {
    pub fn decode(opcode: u8) -> Instruction {
        match opcode {
            0x00 => Instruction::nop(),
            0x10 => Instruction::stop(),
            0x01 | 0x11 | 0x21 | 0x31 => Instruction::ld_rr_nn(opcode),
            0x02 | 0x12 | 0x22 | 0x32 => Instruction::ld_rrm_nn(opcode),
            0x03 | 0x13 | 0x23 | 0x33 => Instruction::inc_rr(opcode),
            0x04 | 0x14 | 0x24 | 0x34 | 0x0C | 0x1C | 0x2C | 0x3C => Instruction::inc_r(opcode),
            0x05 | 0x15 | 0x25 | 0x35 | 0x0D | 0x1D | 0x2D | 0x3D => Instruction::dec_r(opcode),
            0x06 | 0x16 | 0x26 | 0x36 | 0x0E | 0x1E | 0x2E | 0x3E => Instruction::ld_r_n(opcode),
            0x09 | 0x19 | 0x29 | 0x39 => Instruction::add_hl_rr(opcode),
            0x07 => Instruction::rlca(),
            0x17 => Instruction::rla(),
            0x27 => Instruction::daa(),
            0x37 => Instruction::scf(),
            0x0F => Instruction::rrca(),
            0x1F => Instruction::rra(),
            0x2F => Instruction::cpl(),
            0x3F => Instruction::ccf(),
            0x08 => Instruction::ld_nnm_sp(),
            0x18 => Instruction::jr(),
            0x20 | 0x30 | 0x28 | 0x38 => Instruction::jr_cc(opcode),
            0x40..=0x75 | 0x77..=0x7F => Instruction::ld_r_r(opcode),
            0x76 => Instruction::halt(),
            0x80..=0x87 | 0xC6 => Instruction::add_r_r(opcode),
            0x88..=0x8F | 0xCE => Instruction::adc_a_r(opcode),
            0x90..=0x97 | 0xD6 => Instruction::sub_a_r(opcode),
            0x98..=0x9F | 0xDE => Instruction::sbc_a_r(opcode),
            0xA0..=0xA7 | 0xE6 => Instruction::and(opcode),
            0xA8..=0xAF | 0xEE => Instruction::xor(opcode),
            0xB0..=0xB7 | 0xF6 => Instruction::or(opcode),
            0xB8..=0xBF | 0xFE => Instruction::cp(opcode),
            0xC5 | 0xD5 | 0xE5 | 0xF5 => Instruction::push(opcode),
            0xC1 | 0xD1 | 0xE1 | 0xF1 => Instruction::pop(opcode),
            0xC7 | 0xD7 | 0xE7 | 0xF7 | 0xCF | 0xDF | 0xEF | 0xFF => Instruction::rst(opcode),
            0xFB => Instruction::enable_interrupts(),
            0xF3 => Instruction::disable_interrupts(),
            0xC9 => Instruction::ret(),
            _ => todo!("opcode {}", opcode),
        }
    }

    pub fn nop() -> Instruction {
        Instruction {
            steps: vec![Step::with_ops(vec![])],
            ..Default::default()
        }
    }

    pub fn stop() -> Instruction {
        Instruction {
            steps: vec![Step::with_ops(vec![Op::Stop])],
            ..Default::default()
        }
    }

    pub fn halt() -> Instruction {
        Instruction {
            steps: vec![Step::with_ops(vec![Op::Halt])],
            ..Default::default()
        }
    }

    pub fn enable_interrupts() -> Instruction {
        Instruction {
            steps: vec![Step::with_ops(vec![Op::EI])],
            ..Default::default()
        }
    }

    pub fn disable_interrupts() -> Instruction {
        Instruction {
            steps: vec![Step::with_ops(vec![Op::DI])],
            ..Default::default()
        }
    }

    pub fn ret() -> Instruction {
        Instruction {
            cycles: (16, 0),
            len: 1,
            steps: vec![
                Step::with_ops(vec![
                    Op::AssertPostInc(Register::SP),
                    Op::Load(Load::Register(Register::NnLo, Register::Memory)),
                ]),
                Step::with_ops(vec![
                    Op::AssertPostInc(Register::SP),
                    Op::Load(Load::Register(Register::NnHi, Register::Memory)),
                ]),
                Step::with_ops(vec![Op::Load(Load::Register(Register::PC, Register::NN))]),
            ],
            ..Default::default()
        }
    }

    pub fn push(opcode: u8) -> Instruction {
        let (hi, lo) = match opcode {
            0xC5 => (Register::B, Register::C),
            0xD5 => (Register::D, Register::E),
            0xE5 => (Register::H, Register::L),
            0xF5 => (Register::A, Register::F),
            _ => panic!("invalid opcode"),
        };
        Instruction {
            steps: vec![
                Step::with_ops(vec![Op::NOP]),
                Step::with_ops(vec![
                    Op::AssertPreDec(Register::SP),
                    Op::Load(Load::Memory(hi)),
                ]),
                Step::with_ops(vec![
                    Op::AssertPreDec(Register::SP),
                    Op::Load(Load::Memory(lo)),
                ]),
            ],
            ..Default::default()
        }
    }

    pub fn pop(opcode: u8) -> Instruction {
        let (hi, lo) = match opcode {
            0xC1 => (Register::B, Register::C),
            0xD1 => (Register::D, Register::E),
            0xE1 => (Register::H, Register::L),
            0xF1 => (Register::A, Register::F),
            _ => panic!("invalid opcode"),
        };
        Instruction {
            steps: vec![
                Step::with_ops(vec![
                    Op::AssertPreInc(Register::SP),
                    Op::Load(Load::Memory(hi)),
                ]),
                Step::with_ops(vec![
                    Op::AssertPreInc(Register::SP),
                    Op::Load(Load::Memory(lo)),
                ]),
            ],
            ..Default::default()
        }
    }

    pub fn rst(opcode: u8) -> Instruction {
        let rst_addr = match opcode {
            0xC7 => 0x00,
            0xD7 => 0x10,
            0xE7 => 0x20,
            0xF7 => 0x30,
            0xCF => 0x08,
            0xDF => 0x18,
            0xEF => 0x28,
            0xFF => 0x38,
            _ => panic!("invalid opcode"),
        };
        Instruction {
            steps: vec![
                Step::with_ops(vec![Op::SetN(rst_addr)]),
                Step::with_ops(vec![
                    Op::AssertPreDec(Register::SP),
                    Op::Load(Load::MemoryHi(Register::PC)),
                ]),
                Step::with_ops(vec![
                    Op::AssertPreDec(Register::SP),
                    Op::Load(Load::MemoryLo(Register::PC)),
                    Op::Load(Load::Register(Register::PC, Register::N)),
                ]),
            ],
            ..Default::default()
        }
    }

    fn ld_rr_nn(opcode: u8) -> Instruction {
        let r = match opcode {
            0x01 => Register::BC,
            0x11 => Register::DE,
            0x21 => Register::HL,
            0x31 => Register::SP,
            _ => panic!("invalid opcode"),
        };
        Instruction {
            cycles: (12, 0),
            len: 3,
            steps: vec![
                Step::with_ops(vec![Op::Fetch(Fetch::NnLo)]),
                Step::with_ops(vec![
                    Op::Fetch(Fetch::NnHi),
                    Op::Load(Load::Register(r, Register::NN)),
                ]),
            ],
            ..Default::default()
        }
    }

    fn ld_r_r(opcode: u8) -> Instruction {
        let dest = match opcode {
            0x40..=0x47 => Register::B,
            0x48..=0x4F => Register::C,
            0x50..=0x57 => Register::D,
            0x58..=0x5F => Register::E,
            0x60..=0x67 => Register::H,
            0x68..=0x6F => Register::L,
            0x70..=0x75 | 0x77 => Register::HL,
            0x78..=0x7F => Register::A,
            _ => panic!("invalid opcode"),
        };
        let src = match opcode & 0xF {
            0x0 | 0x8 => Register::B,
            0x1 | 0x9 => Register::C,
            0x2 | 0xA => Register::D,
            0x3 | 0xB => Register::E,
            0x4 | 0xC => Register::H,
            0x5 | 0xD => Register::L,
            0x6 | 0xE => Register::HL,
            0x7 | 0xF => Register::A,
            _ => panic!("invalid opcode"),
        };

        if src == Register::HL {
            Instruction {
                cycles: (8, 0),
                len: 1,
                steps: vec![Step::with_ops(vec![
                    Op::Assert(src),
                    Op::Load(Load::Register(dest, Register::Memory)),
                ])],
                ..Default::default()
            }
        } else if dest == Register::HL {
            Instruction {
                cycles: (8, 0),
                len: 1,
                steps: vec![Step::with_ops(vec![
                    Op::Assert(dest),
                    Op::Load(Load::Memory(src)),
                ])],
                ..Default::default()
            }
        } else {
            Instruction {
                cycles: (4, 0),
                len: 1,
                eager: true,
                steps: vec![Step::with_ops(vec![Op::Load(Load::Register(dest, src))])],
                ..Default::default()
            }
        }
    }

    fn ld_rrm_nn(opcode: u8) -> Instruction {
        let r = match opcode {
            0x02 => Register::BC,
            0x12 => Register::DE,
            0x22 => Register::HLI,
            0x32 => Register::HLD,
            _ => panic!("invalid opcode"),
        };
        Instruction {
            cycles: (8, 0),
            len: 1,
            steps: vec![Step::with_ops(vec![
                Op::Assert(r),
                Op::Load(Load::Memory(Register::A)),
            ])],
            ..Default::default()
        }
    }

    fn ld_nnm_sp() -> Instruction {
        let r = Register::SP;
        Instruction {
            cycles: (20, 0),
            len: 3,
            steps: vec![
                Step::with_ops(vec![Op::Fetch(Fetch::NnLo)]),
                Step::with_ops(vec![Op::Fetch(Fetch::NnHi)]),
                Step::with_ops(vec![Op::Assert(Register::NN), Op::Load(Load::MemoryLo(r))]),
                Step::with_ops(vec![
                    Op::AssertInc(Register::NN),
                    Op::Load(Load::MemoryHi(r)),
                ]),
            ],
            ..Default::default()
        }
    }

    fn inc_rr(opcode: u8) -> Instruction {
        let r = match opcode {
            0x03 => Register::BC,
            0x13 => Register::DE,
            0x23 => Register::HL,
            0x33 => Register::SP,
            _ => panic!("invalid opcode"),
        };
        Instruction {
            cycles: (8, 0),
            len: 1,
            steps: vec![Step::with_ops(vec![Op::Inc(Inc::Register(r))])],
            ..Default::default()
        }
    }

    fn inc_r(opcode: u8) -> Instruction {
        let r = match opcode {
            0x04 => Register::B,
            0x14 => Register::D,
            0x24 => Register::H,
            0x34 => Register::HL,
            0x0C => Register::C,
            0x1C => Register::E,
            0x2C => Register::L,
            0x3C => Register::A,
            _ => panic!("invalid opcode"),
        };
        let mut i = Instruction {
            cycles: (4, 0),
            len: 1,
            eager: true,
            ..Default::default()
        };
        if r == Register::HL {
            i.cycles = (12, 0);
            i.eager = false;
            i.steps.push(Step::with_ops(vec![Op::Assert(Register::HL)]));
            i.steps.push(Step::with_ops(vec![Op::Inc(Inc::Memory)]));
        } else {
            i.steps
                .push(Step::with_ops(vec![Op::Inc(Inc::Register(r))]));
        }
        i
    }

    fn dec_r(opcode: u8) -> Instruction {
        let r = match opcode {
            0x05 => Register::B,
            0x15 => Register::D,
            0x25 => Register::H,
            0x35 => Register::HL,
            0x0D => Register::C,
            0x1D => Register::E,
            0x2D => Register::L,
            0x3D => Register::A,
            _ => panic!("invalid opcode"),
        };
        let mut i = Instruction {
            cycles: (4, 0),
            len: 1,
            eager: true,
            ..Default::default()
        };
        if r == Register::HL {
            i.cycles = (12, 0);
            i.eager = false;
            i.steps.push(Step::with_ops(vec![Op::Assert(Register::HL)]));
            i.steps.push(Step::with_ops(vec![Op::Dec(Dec::Memory)]));
        } else {
            i.steps
                .push(Step::with_ops(vec![Op::Dec(Dec::Register(r))]));
        }
        i
    }

    fn ld_r_n(opcode: u8) -> Instruction {
        let r = match opcode {
            0x06 => Register::B,
            0x16 => Register::D,
            0x26 => Register::H,
            0x36 => Register::HL,
            0x0E => Register::C,
            0x1E => Register::E,
            0x2E => Register::L,
            0x3E => Register::A,
            _ => panic!("invalid opcode"),
        };
        let mut i = Instruction {
            cycles: (8, 0),
            len: 2,
            ..Default::default()
        };
        if r == Register::HL {
            i.cycles = (12, 0);
            i.steps.push(Step::with_ops(vec![Op::Fetch(Fetch::N)]));
            i.steps.push(Step::with_ops(vec![
                Op::Assert(Register::HL),
                Op::Load(Load::Memory(Register::N)),
            ]))
        } else {
            i.steps.push(Step::with_ops(vec![
                Op::Fetch(Fetch::N),
                Op::Load(Load::Register(r, Register::N)),
            ]));
        }
        i
    }

    fn rlca() -> Instruction {
        Instruction {
            cycles: (4, 0),
            len: 1,
            steps: vec![Step::with_ops(vec![Op::RLC(Register::A)])],
            eager: true,
            ..Default::default()
        }
    }

    fn rla() -> Instruction {
        Instruction {
            cycles: (4, 0),
            len: 1,
            steps: vec![Step::with_ops(vec![Op::RL(Register::A)])],
            eager: true,
            ..Default::default()
        }
    }

    fn rrca() -> Instruction {
        Instruction {
            cycles: (4, 0),
            len: 1,
            steps: vec![Step::with_ops(vec![Op::RRC(Register::A)])],
            eager: true,
            ..Default::default()
        }
    }

    fn rra() -> Instruction {
        Instruction {
            cycles: (4, 0),
            len: 1,
            steps: vec![Step::with_ops(vec![Op::RR(Register::A)])],
            eager: true,
            ..Default::default()
        }
    }

    fn daa() -> Instruction {
        Instruction {
            cycles: (4, 0),
            len: 1,
            steps: vec![Step::with_ops(vec![Op::DAA])],
            eager: true,
            ..Default::default()
        }
    }

    fn scf() -> Instruction {
        Instruction {
            cycles: (4, 0),
            len: 1,
            steps: vec![Step::with_ops(vec![Op::SCF])],
            eager: true,
            ..Default::default()
        }
    }

    fn cpl() -> Instruction {
        Instruction {
            cycles: (4, 0),
            len: 1,
            steps: vec![Step::with_ops(vec![Op::CPL])],
            eager: true,
            ..Default::default()
        }
    }

    fn ccf() -> Instruction {
        Instruction {
            cycles: (4, 0),
            len: 1,
            steps: vec![Step::with_ops(vec![Op::CCF])],
            eager: true,
            ..Default::default()
        }
    }

    fn jr() -> Instruction {
        Instruction {
            cycles: (12, 0),
            len: 2,
            steps: vec![
                Step::with_ops(vec![Op::Fetch(Fetch::E)]),
                Step::with_ops(vec![Op::Add(Add::Register(Register::PC, Register::NE))]),
            ],
            ..Default::default()
        }
    }

    fn jr_cc(opcode: u8) -> Instruction {
        let cond = match opcode {
            0x20 => Cond::NZ,
            0x30 => Cond::NC,
            0x28 => Cond::Z,
            0x38 => Cond::C,
            _ => panic!("invalid opcode {}", opcode),
        };
        Instruction {
            cycles: (8, 12),
            len: 2,
            steps: vec![
                Step::with_ops(vec![Op::Fetch(Fetch::E)]),
                Step::with_ops(vec![
                    Op::CheckCond(cond),
                    Op::Add(Add::Register(Register::PC, Register::NE)),
                ]),
            ],
            ..Default::default()
        }
    }

    fn add_hl_rr(opcode: u8) -> Instruction {
        let r = match opcode {
            0x09 => Register::BC,
            0x19 => Register::DE,
            0x29 => Register::HL,
            0x39 => Register::SP,
            _ => panic!("invalid opcode {}", opcode),
        };
        Instruction {
            cycles: (8, 0),
            len: 1,
            steps: vec![Step::with_ops(vec![Op::Add(Add::Register(
                Register::HL,
                r,
            ))])],
            ..Default::default()
        }
    }

    fn adc_a_r(opcode: u8) -> Instruction {
        let dest = Register::A;
        let src = match opcode {
            0xCE => Register::N,
            _ => match opcode & 0xF {
                0x0 | 0x8 => Register::B,
                0x1 | 0x9 => Register::C,
                0x2 | 0xA => Register::D,
                0x3 | 0xB => Register::E,
                0x4 | 0xC => Register::H,
                0x5 | 0xD => Register::L,
                0x6 | 0xE => Register::HL,
                0x7 | 0xF => Register::A,
                _ => panic!("invalid opcode {}", opcode),
            },
        };

        if src == Register::HL {
            Instruction {
                cycles: (8, 0),
                len: 1,
                steps: vec![Step::with_ops(vec![
                    Op::Assert(src),
                    Op::ADC(ADC::Register(dest, Register::Memory)),
                ])],
                ..Default::default()
            }
        } else if src == Register::N {
            Instruction {
                cycles: (8, 0),
                len: 2,
                steps: vec![Step::with_ops(vec![
                    Op::Fetch(Fetch::N),
                    Op::ADC(ADC::Register(dest, src)),
                ])],
                ..Default::default()
            }
        } else {
            Instruction {
                cycles: (4, 0),
                len: 1,
                eager: true,
                steps: vec![Step::with_ops(vec![Op::ADC(ADC::Register(dest, src))])],
                ..Default::default()
            }
        }
    }

    fn add_r_r(opcode: u8) -> Instruction {
        let dest = Register::A;
        let src = match opcode {
            0xC6 => Register::N,
            _ => match opcode & 0xF {
                0x0 | 0x8 => Register::B,
                0x1 | 0x9 => Register::C,
                0x2 | 0xA => Register::D,
                0x3 | 0xB => Register::E,
                0x4 | 0xC => Register::H,
                0x5 | 0xD => Register::L,
                0x6 | 0xE => Register::HL,
                0x7 | 0xF => Register::A,
                _ => panic!("invalid opcode {}", opcode),
            },
        };

        if src == Register::HL {
            Instruction {
                cycles: (8, 0),
                len: 1,
                steps: vec![Step::with_ops(vec![
                    Op::Assert(src),
                    Op::Add(Add::Register(dest, Register::Memory)),
                ])],
                ..Default::default()
            }
        } else if src == Register::N {
            Instruction {
                cycles: (8, 0),
                len: 2,
                steps: vec![Step::with_ops(vec![
                    Op::Fetch(Fetch::N),
                    Op::Add(Add::Register(dest, src)),
                ])],
                ..Default::default()
            }
        } else {
            Instruction {
                cycles: (4, 0),
                len: 1,
                eager: true,
                steps: vec![Step::with_ops(vec![Op::Add(Add::Register(dest, src))])],
                ..Default::default()
            }
        }
    }

    fn sub_a_r(opcode: u8) -> Instruction {
        let dest = Register::A;
        let src = match opcode {
            0xD6 => Register::N,
            _ => match opcode & 0xF {
                0x0 | 0x8 => Register::B,
                0x1 | 0x9 => Register::C,
                0x2 | 0xA => Register::D,
                0x3 | 0xB => Register::E,
                0x4 | 0xC => Register::H,
                0x5 | 0xD => Register::L,
                0x6 | 0xE => Register::HL,
                0x7 | 0xF => Register::A,
                _ => panic!("invalid opcode {}", opcode),
            },
        };

        if src == Register::HL {
            Instruction {
                cycles: (8, 0),
                len: 1,
                steps: vec![Step::with_ops(vec![
                    Op::Assert(src),
                    Op::Sub(Sub::Register(dest, Register::Memory)),
                ])],
                ..Default::default()
            }
        } else if src == Register::N {
            Instruction {
                cycles: (8, 0),
                len: 2,
                steps: vec![Step::with_ops(vec![
                    Op::Fetch(Fetch::N),
                    Op::Sub(Sub::Register(dest, src)),
                ])],
                ..Default::default()
            }
        } else {
            Instruction {
                cycles: (4, 0),
                len: 1,
                eager: true,
                steps: vec![Step::with_ops(vec![Op::Sub(Sub::Register(dest, src))])],
                ..Default::default()
            }
        }
    }

    fn sbc_a_r(opcode: u8) -> Instruction {
        let dest = Register::A;
        let src = match opcode {
            0xDE => Register::N,
            _ => match opcode & 0xF {
                0x0 | 0x8 => Register::B,
                0x1 | 0x9 => Register::C,
                0x2 | 0xA => Register::D,
                0x3 | 0xB => Register::E,
                0x4 | 0xC => Register::H,
                0x5 | 0xD => Register::L,
                0x6 | 0xE => Register::HL,
                0x7 | 0xF => Register::A,
                _ => panic!("invalid opcode {}", opcode),
            },
        };

        if src == Register::HL {
            Instruction {
                cycles: (8, 0),
                len: 1,
                steps: vec![Step::with_ops(vec![
                    Op::Assert(src),
                    Op::SBC(SBC::Register(dest, Register::Memory)),
                ])],
                ..Default::default()
            }
        } else if src == Register::N {
            Instruction {
                cycles: (8, 0),
                len: 2,
                steps: vec![Step::with_ops(vec![
                    Op::Fetch(Fetch::N),
                    Op::SBC(SBC::Register(dest, src)),
                ])],
                ..Default::default()
            }
        } else {
            Instruction {
                cycles: (4, 0),
                len: 1,
                eager: true,
                steps: vec![Step::with_ops(vec![Op::SBC(SBC::Register(dest, src))])],
                ..Default::default()
            }
        }
    }

    fn and(opcode: u8) -> Instruction {
        let src = match opcode {
            0xE6 => Register::N,
            _ => match opcode & 0xF {
                0x0 | 0x8 => Register::B,
                0x1 | 0x9 => Register::C,
                0x2 | 0xA => Register::D,
                0x3 | 0xB => Register::E,
                0x4 | 0xC => Register::H,
                0x5 | 0xD => Register::L,
                0x6 | 0xE => Register::HL,
                0x7 | 0xF => Register::A,
                _ => panic!("invalid opcode {}", opcode),
            },
        };

        if src == Register::HL {
            Instruction {
                cycles: (8, 0),
                len: 1,
                steps: vec![Step::with_ops(vec![
                    Op::Assert(src),
                    Op::AND(Register::Memory),
                ])],
                ..Default::default()
            }
        } else if src == Register::N {
            Instruction {
                cycles: (8, 0),
                len: 2,
                steps: vec![Step::with_ops(vec![Op::Fetch(Fetch::N), Op::AND(src)])],
                ..Default::default()
            }
        } else {
            Instruction {
                cycles: (4, 0),
                len: 1,
                eager: true,
                steps: vec![Step::with_ops(vec![Op::AND(src)])],
                ..Default::default()
            }
        }
    }

    fn xor(opcode: u8) -> Instruction {
        let src = match opcode {
            0xEE => Register::N,
            _ => match opcode & 0xF {
                0x0 | 0x8 => Register::B,
                0x1 | 0x9 => Register::C,
                0x2 | 0xA => Register::D,
                0x3 | 0xB => Register::E,
                0x4 | 0xC => Register::H,
                0x5 | 0xD => Register::L,
                0x6 | 0xE => Register::HL,
                0x7 | 0xF => Register::A,
                _ => panic!("invalid opcode {}", opcode),
            },
        };

        if src == Register::HL {
            Instruction {
                cycles: (8, 0),
                len: 1,
                steps: vec![Step::with_ops(vec![
                    Op::Assert(src),
                    Op::XOR(Register::Memory),
                ])],
                ..Default::default()
            }
        } else if src == Register::N {
            Instruction {
                cycles: (8, 0),
                len: 2,
                steps: vec![Step::with_ops(vec![Op::Fetch(Fetch::N), Op::XOR(src)])],
                ..Default::default()
            }
        } else {
            Instruction {
                cycles: (4, 0),
                len: 1,
                eager: true,
                steps: vec![Step::with_ops(vec![Op::XOR(src)])],
                ..Default::default()
            }
        }
    }

    fn or(opcode: u8) -> Instruction {
        let src = match opcode {
            0xF6 => Register::N,
            _ => match opcode & 0xF {
                0x0 | 0x8 => Register::B,
                0x1 | 0x9 => Register::C,
                0x2 | 0xA => Register::D,
                0x3 | 0xB => Register::E,
                0x4 | 0xC => Register::H,
                0x5 | 0xD => Register::L,
                0x6 | 0xE => Register::HL,
                0x7 | 0xF => Register::A,
                _ => panic!("invalid opcode {}", opcode),
            },
        };

        if src == Register::HL {
            Instruction {
                cycles: (8, 0),
                len: 1,
                steps: vec![Step::with_ops(vec![
                    Op::Assert(src),
                    Op::OR(Register::Memory),
                ])],
                ..Default::default()
            }
        } else if src == Register::N {
            Instruction {
                cycles: (8, 0),
                len: 2,
                steps: vec![Step::with_ops(vec![Op::Fetch(Fetch::N), Op::OR(src)])],
                ..Default::default()
            }
        } else {
            Instruction {
                cycles: (4, 0),
                len: 1,
                eager: true,
                steps: vec![Step::with_ops(vec![Op::OR(src)])],
                ..Default::default()
            }
        }
    }

    fn cp(opcode: u8) -> Instruction {
        let src = match opcode {
            0xFE => Register::N,
            _ => match opcode & 0xF {
                0x0 | 0x8 => Register::B,
                0x1 | 0x9 => Register::C,
                0x2 | 0xA => Register::D,
                0x3 | 0xB => Register::E,
                0x4 | 0xC => Register::H,
                0x5 | 0xD => Register::L,
                0x6 | 0xE => Register::HL,
                0x7 | 0xF => Register::A,
                _ => panic!("invalid opcode {}", opcode),
            },
        };

        if src == Register::HL {
            Instruction {
                cycles: (8, 0),
                len: 1,
                steps: vec![Step::with_ops(vec![
                    Op::Assert(src),
                    Op::CP(Register::Memory),
                ])],
                ..Default::default()
            }
        } else if src == Register::N {
            Instruction {
                cycles: (8, 0),
                len: 2,
                steps: vec![Step::with_ops(vec![Op::Fetch(Fetch::N), Op::CP(src)])],
                ..Default::default()
            }
        } else {
            Instruction {
                cycles: (4, 0),
                len: 1,
                eager: true,
                steps: vec![Step::with_ops(vec![Op::CP(src)])],
                ..Default::default()
            }
        }
    }
}

impl Default for Instruction {
    fn default() -> Self {
        Instruction {
            opcode: 0,
            len: 1,
            cycles: (4, 0),
            n8: None,
            e: None,
            n16_lo: None,
            n16_hi: None,
            steps: vec![],
            eager: false,
            branched: false,
            done: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum R8 {
    B,
    C,
    D,
    E,
    H,
    L,
    HL,
    A,
    N8,
}

pub const R8VALUES: [R8; 9] = [
    R8::B,
    R8::C,
    R8::D,
    R8::E,
    R8::H,
    R8::L,
    R8::HL,
    R8::A,
    R8::N8,
];

impl TryFrom<u8> for R8 {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(R8::B),
            1 => Ok(R8::C),
            2 => Ok(R8::D),
            3 => Ok(R8::E),
            4 => Ok(R8::H),
            5 => Ok(R8::L),
            6 => Ok(R8::HL),
            7 => Ok(R8::A),
            _ => Err(Error::InvalidR8(value)),
        }
    }
}

impl Display for R8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r8: &'static str = match self {
            R8::B => "b",
            R8::C => "c",
            R8::D => "d",
            R8::E => "e",
            R8::H => "h",
            R8::L => "l",
            R8::HL => "[hl]",
            R8::A => "a",
            R8::N8 => "n8",
        };
        write!(f, "{}", r8)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mem {
    HL,
    BC,
    DE,
    SP,
    HLI,
    HLD,
    SPN8,
    N16,
    N8,
    C,
}

impl Mem {
    pub fn r16mem(value: u8) -> Result<Mem, Error> {
        match value {
            0 => Ok(Mem::BC),
            1 => Ok(Mem::DE),
            2 => Ok(Mem::HLI),
            3 => Ok(Mem::HLD),
            _ => Err(Error::InvalidR16Mem(value).into()),
        }
    }
}

impl Display for Mem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Mem::C => "[c]",
            Mem::N16 => "[n16]",
            Mem::N8 => "[n8]",
            Mem::DE => "[de]",
            Mem::BC => "[bc]",
            Mem::SP => "[sp]",
            Mem::HL => "[hl]",
            Mem::HLI => "[hl+]",
            Mem::HLD => "[hl-]",
            Mem::SPN8 => "sp + n8",
        };
        write!(f, "{}", String::from(s))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum R16 {
    BC,
    DE,
    HL,
    SP,
    AF,
    PC,
    N16,
}

impl R16 {
    pub fn r16stk(v: u8) -> Result<R16, Error> {
        match v {
            0 => Ok(R16::BC),
            1 => Ok(R16::DE),
            2 => Ok(R16::HL),
            3 => Ok(R16::AF),
            _ => Err(Error::InvalidR16Stk(v).into()),
        }
    }
}

impl TryFrom<u8> for R16 {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(R16::BC),
            1 => Ok(R16::DE),
            2 => Ok(R16::HL),
            3 => Ok(R16::SP),
            _ => Err(Error::InvalidR16(value).into()),
        }
    }
}

impl Display for R16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r16: &'static str = match self {
            R16::BC => "bc",
            R16::DE => "de",
            R16::HL => "hl",
            R16::SP => "sp",
            R16::AF => "af",
            R16::PC => "pc",
            R16::N16 => "n16",
        };
        write!(f, "{}", r16)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cond {
    NZ,
    Z,
    NC,
    C,
}

impl TryFrom<u8> for Cond {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Cond::NZ),
            1 => Ok(Cond::Z),
            2 => Ok(Cond::NC),
            3 => Ok(Cond::C),
            _ => Err(Error::InvalidCond(value).into()),
        }
    }
}

impl Display for Cond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r16: &'static str = match self {
            Cond::NZ => "nz",
            Cond::Z => "z",
            Cond::NC => "nc",
            Cond::C => "c",
        };
        write!(f, "{}", r16)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct B3(u8);

impl B3 {
    pub fn val(&self) -> u8 {
        self.0
    }
}

impl From<u8> for B3 {
    fn from(value: u8) -> Self {
        B3(value & 0x3)
    }
}

impl Display for B3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct T3(u8);

impl T3 {
    pub fn val(&self) -> u8 {
        self.0
    }
}

impl From<u8> for T3 {
    fn from(value: u8) -> Self {
        T3(value * 8)
    }
}

impl From<T3> for u8 {
    fn from(value: T3) -> Self {
        value.0
    }
}

impl Display for T3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:0>2x}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Step {
    ops: Vec<Op>,
    done: bool,
}

impl Step {
    pub fn with_ops(ops: Vec<Op>) -> Step {
        Step { ops, done: false }
    }

    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    pub fn set_done(&mut self) {
        self.done = true
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Op {
    NOP,
    Add(Add),
    Sub(Sub),
    ADC(ADC),
    SBC(SBC),
    AND(Register),
    XOR(Register),
    OR(Register),
    CP(Register),
    Fetch(Fetch),
    Assert(Register),
    AssertInc(Register),
    AssertPostInc(Register),
    AssertPreDec(Register),
    AssertPreInc(Register),
    SetN(u8),
    Load(Load),
    Inc(Inc),
    Dec(Dec),
    RLC(Register),
    RL(Register),
    RRC(Register),
    RR(Register),
    CheckCond(Cond),
    Stop,
    Halt,
    DAA,
    SCF,
    CPL,
    CCF,
    EI,
    DI,
}

#[derive(Debug, Clone, Copy)]
pub enum Fetch {
    N,
    NnLo,
    NnHi,
    E,
}

#[derive(Debug, Clone, Copy)]
pub enum Add {
    Memory(Register, Register),
    Register(Register, Register),
    RegisterLo(Register, Register),
    RegisterHi(Register, Register),
}

#[derive(Debug, Clone, Copy)]
pub enum Sub {
    Memory(Register, Register),
    Register(Register, Register),
}

#[derive(Debug, Clone, Copy)]
pub enum ADC {
    Memory(Register, Register),
    Register(Register, Register),
}

#[derive(Debug, Clone, Copy)]
pub enum SBC {
    Memory(Register, Register),
    Register(Register, Register),
}

#[derive(Debug, Clone, Copy)]
pub enum Dec {
    Memory,
    Register(Register),
}

#[derive(Debug, Clone, Copy)]
pub enum Inc {
    Memory,
    Register(Register),
}

#[derive(Debug, Clone, Copy)]
pub enum Load {
    Register(Register, Register),
    Memory(Register),
    MemoryLo(Register),
    MemoryHi(Register),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Register {
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
    BC,
    HL,
    HLI,
    HLD,
    DE,
    AF,
    SP,
    SPDec,
    PC,
    N,
    NN,
    NnLo,
    NnHi,
    NE,
    Memory,
}

impl Register {
    pub fn is_byte(&self) -> bool {
        match self {
            Self::A
            | Self::B
            | Self::C
            | Self::D
            | Self::E
            | Self::F
            | Self::H
            | Self::L
            | Self::N
            | Self::NE
            | Self::NnLo
            | Self::NnHi
            | Self::Memory => true,
            _ => false,
        }
    }

    pub fn is_word(&self) -> bool {
        match self {
            Self::BC | Self::HL | Self::DE | Self::AF | Self::SP | Self::PC | Self::NN => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ADD {
    A(R8),
    HL(R16),
    SP,
}

impl Display for ADD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::from("add ");
        match self {
            ADD::A(r) => s.push_str(format!("a, {}", r).as_str()),
            ADD::HL(r) => s.push_str(format!("hl, {}", r).as_str()),
            ADD::SP => s.push_str("sp, e8"),
        }
        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LD {
    R8(R8, R8),
    R16(R16, R16),
    MemR8(Mem, R8),
    R8Mem(R8, Mem),
    MemR16(Mem, R16),
    HLSPN,
}

impl Display for LD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::from("ld ");
        match self {
            LD::R16(a, b) => s.push_str(format!("{}, {}", a, b).as_str()),
            LD::R8(a, b) => s.push_str(format!("{}, {}", a, b).as_str()),
            LD::MemR8(a, b) => s.push_str(format!("{}, {}", a, b).as_str()),
            LD::R8Mem(a, b) => s.push_str(format!("{}, {}", a, b).as_str()),
            LD::MemR16(a, b) => s.push_str(format!("{}, {}", a, b).as_str()),
            LD::HLSPN => s.push_str("hl, sp + n8"),
        }
        write!(f, "{}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LDH {
    A(Mem),
    Mem(Mem),
}

impl Display for LDH {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LDH::A(m) => format!("a, {}", m),
            LDH::Mem(m) => format!("{}, a", m),
        };
        write!(f, "ldh {}", s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum INC {
    R8(R8),
    R16(R16),
}

impl Display for INC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let o = match self {
            INC::R8(r) => format!("{}", r),
            INC::R16(r) => format!("{}", r),
        };
        write!(f, "inc {}", o)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DEC {
    R8(R8),
    R16(R16),
}

impl Display for DEC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let o = match self {
            DEC::R8(r) => format!("{}", r),
            DEC::R16(r) => format!("{}", r),
        };
        write!(f, "dec {}", o)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JR {
    N8,
    Cond(Cond),
}

impl Display for JR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            JR::N8 => String::from("n8"),
            JR::Cond(c) => format!("{}, n8", c),
        };
        write!(f, "jr {}", s)
    }
}
