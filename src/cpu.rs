use crate::registers::{ProgramCounter, Register};

pub struct CPU {
    rf: RegisterFile,
    sp: Register,
    pc: ProgramCounter,
}

pub struct RegisterFile {
    af: Register,
    bc: Register,
    de: Register,
    hl: Register,
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            rf: RegisterFile::new(),
            sp: Register::new(),
            pc: ProgramCounter::new(),
        }
    }

    pub fn rf(&self) -> &RegisterFile {
        &self.rf
    }

    pub fn rf_mut(&mut self) -> &mut RegisterFile {
        &mut self.rf
    }
}

impl RegisterFile {
    pub fn new() -> RegisterFile {
        RegisterFile {
            af: Register::new(),
            bc: Register::new(),
            de: Register::new(),
            hl: Register::new(),
        }
    }

    pub fn flags(&self) -> u8 {
        self.af().low()
    }

    pub fn af(&self) -> &Register {
        &self.af
    }

    pub fn af_mut(&mut self) -> &mut Register {
        &mut self.af
    }

    pub fn bc(&self) -> &Register {
        &self.bc
    }

    pub fn bc_mut(&mut self) -> &mut Register {
        &mut self.bc
    }

    pub fn de(&self) -> &Register {
        &self.de
    }

    pub fn de_mut(&mut self) -> &mut Register {
        &mut self.de
    }

    pub fn hl(&self) -> &Register {
        &self.hl
    }

    pub fn hl_mut(&mut self) -> &mut Register {
        &mut self.hl
    }
}
