use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct RegisterIndex {
    value: u32,
}

impl From<u32> for RegisterIndex {
    fn from(value: u32) -> Self {
        RegisterIndex { value }
    }
}

impl From<RegisterIndex> for u32 {
    fn from(value: RegisterIndex) -> Self {
        value.value
    }
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
    pub fn inc(&mut self) -> Self {
        let prev = *self;
        self.value += 1;
        prev
    }
}

#[derive(Debug)]
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

pub struct CompiledFunction<'input> {
    pub name: &'input str,
    pub body: Vec<Instruction>,
    pub max_used_registers: RegisterIndex,
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
        writeln!(f, "fn {} - #reg: {} {{", self.name, self.max_used_registers)?;
        for (i, instr) in self.body.iter().enumerate() {
            writeln!(f, "  {:-3}:  {}", i, instr)?;
        }
        write!(f, "}}")
    }
}
