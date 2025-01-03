use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct RegisterIndex {
    value: u32,
}

impl From<RegisterIndex> for usize {
    fn from(value: RegisterIndex) -> Self {
        value.value as usize
    }
}

impl fmt::Display for RegisterIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl RegisterIndex {
    pub const fn from_u32(value: u32) -> Self {
        RegisterIndex { value }
    }

    pub fn inc(&mut self) -> Self {
        let prev = *self;
        self.value += 1;
        prev
    }
}

#[derive(Debug, PartialEq)]
pub enum Instruction {
    // Move immediate
    Mvi {
        dest: RegisterIndex,
        val: f64,
    },
    Add {
        dest: RegisterIndex,
        op1: RegisterIndex,
        op2: RegisterIndex,
    },
    Sub {
        dest: RegisterIndex,
        op1: RegisterIndex,
        op2: RegisterIndex,
    },
    Mul {
        dest: RegisterIndex,
        op1: RegisterIndex,
        op2: RegisterIndex,
    },
    Div {
        dest: RegisterIndex,
        op1: RegisterIndex,
        op2: RegisterIndex,
    },

    Ret {
        reg: RegisterIndex,
    },
}

impl Instruction {
    pub fn operands(&self) -> impl Iterator<Item = RegisterIndex> {
        match self {
            Instruction::Mvi { dest, val: _ } => vec![*dest].into_iter(),
            Instruction::Add { dest, op1, op2 } => vec![*dest, *op1, *op2].into_iter(),
            Instruction::Sub { dest, op1, op2 } => vec![*dest, *op1, *op2].into_iter(),
            Instruction::Mul { dest, op1, op2 } => vec![*dest, *op1, *op2].into_iter(),
            Instruction::Div { dest, op1, op2 } => vec![*dest, *op1, *op2].into_iter(),
            Instruction::Ret { reg } => vec![*reg].into_iter(),
        }
    }
}

#[derive(Debug)]
pub struct CompiledFunction<'input> {
    pub name: &'input str,
    pub body: Vec<Instruction>,
    pub num_used_registers: usize,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Mvi { dest, val } => write!(f, "mvi @r{}, {}", dest, val),
            Instruction::Add { dest, op1, op2 } => write!(f, "add @r{}, r{}, r{}", dest, op1, op2),
            Instruction::Sub { dest, op1, op2 } => write!(f, "sub @r{}, r{}, r{}", dest, op1, op2),
            Instruction::Mul { dest, op1, op2 } => write!(f, "mul @r{}, r{}, r{}", dest, op1, op2),
            Instruction::Div { dest, op1, op2 } => write!(f, "div @r{}, r{}, r{}", dest, op1, op2),
            Instruction::Ret { reg } => write!(f, "ret r{}", reg),
        }
    }
}

impl fmt::Display for CompiledFunction<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "fn {} - #reg: {} {{", self.name, self.num_used_registers)?;
        for (i, instr) in self.body.iter().enumerate() {
            writeln!(f, "  {:-3}:  {}", i, instr)?;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
pub mod builders {
    use super::*;

    pub fn mvi(dest: u32, val: f64) -> Instruction {
        Instruction::Mvi {
            dest: RegisterIndex::from_u32(dest),
            val,
        }
    }

    pub fn add(dest: u32, op1: u32, op2: u32) -> Instruction {
        Instruction::Add {
            dest: RegisterIndex::from_u32(dest),
            op1: RegisterIndex::from_u32(op1),
            op2: RegisterIndex::from_u32(op2),
        }
    }

    pub fn sub(dest: u32, op1: u32, op2: u32) -> Instruction {
        Instruction::Sub {
            dest: RegisterIndex::from_u32(dest),
            op1: RegisterIndex::from_u32(op1),
            op2: RegisterIndex::from_u32(op2),
        }
    }

    pub fn mul(dest: u32, op1: u32, op2: u32) -> Instruction {
        Instruction::Mul {
            dest: RegisterIndex::from_u32(dest),
            op1: RegisterIndex::from_u32(op1),
            op2: RegisterIndex::from_u32(op2),
        }
    }

    pub fn div(dest: u32, op1: u32, op2: u32) -> Instruction {
        Instruction::Div {
            dest: RegisterIndex::from_u32(dest),
            op1: RegisterIndex::from_u32(op1),
            op2: RegisterIndex::from_u32(op2),
        }
    }

    pub fn ret(reg: u32) -> Instruction {
        Instruction::Ret {
            reg: RegisterIndex::from_u32(reg),
        }
    }
}
