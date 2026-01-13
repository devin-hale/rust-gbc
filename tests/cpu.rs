use std::sync::{Arc, Mutex};

use gbc::{
    cpu::{
        CPU,
        instr::{LD, Mem, Operation, R8, R16},
    },
    memory::Memory,
};

fn setup() -> (CPU, Arc<Mutex<Memory>>) {
    let mem = Memory::arc();
    let cpu = CPU::new(&mem);
    (cpu, mem)
}

#[test]
fn halt() {
    let opcode = 0b0111_0110;
    let (cpu, _) = setup();
    let i = cpu.decode(opcode).unwrap();
    assert_eq!(i.op(), Operation::HALT);
}

#[test]
fn ld_r_s() {
    for r in 0b000..=0b111 {
        for s in 0b000..=0b111 {
            let opcode = 0b0100_0000 | (r << 3) | s;
            let (mut cpu, _) = setup();
            let i = cpu.decode(opcode).unwrap();
            let a: R8 = r.try_into().unwrap();
            let b: R8 = s.try_into().unwrap();

            if a == R8::HL && b == R8::HL {
                assert_eq!(i.op(), Operation::HALT);
                continue;
            } else if a == R8::HL {
                assert_eq!(i.op(), Operation::LD(LD::MemR8(Mem::HL, b)));
            } else if b == R8::HL {
                assert_eq!(i.op(), Operation::LD(LD::R8Mem(a, Mem::HL)));
            } else {
                assert_eq!(i.op(), Operation::LD(LD::R8(a, b)));
            }

            let val = 0xfe;
            cpu.ld_r8(b, val);
            cpu.execute(i);
            assert_eq!(cpu.src_r8(a), val);
        }
    }
    setup();
}

#[test]
fn push() {
    for qq in 0..=3u8 {
        // 11qq0101
        let (mut cpu, mem) = setup();
        let opcode = 0b1100_0101 | (qq << 4);

        mem.lock().unwrap().write(cpu.pc(), opcode);
        let fetched = cpu.fetch();
        assert_eq!(fetched, opcode);

        let r = R16::r16stk(qq).unwrap();
        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::PUSH(r));

        let val = 0xfe;
        cpu.ld_r16(r, val);
        assert_eq!(cpu.src_r16(r), val);

        let sp = cpu.sp();
        cpu.execute(i);
        assert_eq!(mem.lock().unwrap().read_word(cpu.sp()), val);
        assert_eq!(cpu.sp(), sp - 2);
    }
}

#[test]
fn pop() {
    for qq in 0..=3u8 {
        // 11qq0001
        let (mut cpu, mem) = setup();
        let opcode = 0b1100_0001 | (qq << 4);

        mem.lock().unwrap().write(cpu.pc(), opcode);
        let fetched = cpu.fetch();
        assert_eq!(fetched, opcode);

        let r = R16::r16stk(qq).unwrap();
        let i = cpu.decode(opcode).unwrap();
        assert_eq!(i.op(), Operation::POP(r));

        let val = 0xDEAD;
        cpu.push(val);

        let sp = cpu.sp();
        cpu.execute(i);
        let r16 = cpu.src_r16(r);
        if r == R16::AF {
            assert_eq!(r16, val & 0xFFF0);
        } else {
            assert_eq!(r16, val);
        }
        assert_eq!(cpu.sp(), sp + 2);
    }
}
