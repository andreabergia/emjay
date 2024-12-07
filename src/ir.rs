use core::fmt;

pub type RegisterIndex = u32;

#[derive(Debug)]
pub enum Instruction {
    Mov {
        dest: RegisterIndex,
        val: f64,
    },
    Add {
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
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Mov { dest, val } => write!(f, "mov r{}, {}", dest, val),
            Instruction::Add { dest, op1, op2 } => write!(f, "add r{}, r{}, r{}", dest, op1, op2),
            Instruction::Ret { reg } => write!(f, "ret r{}", reg),
        }
    }
}

impl fmt::Display for CompiledFunction<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "fn {} {{", self.name)?;
        for instr in &self.body {
            writeln!(f, "    {}", instr)?;
        }
        write!(f, "}}")
    }
}
