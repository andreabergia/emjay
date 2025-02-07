use core::fmt;

use crate::frontend::FunctionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct IrRegister {
    value: u32,
}

impl From<IrRegister> for usize {
    fn from(value: IrRegister) -> Self {
        value.value as usize
    }
}

impl fmt::Display for IrRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl IrRegister {
    pub const fn from_u32(value: u32) -> Self {
        IrRegister { value }
    }

    pub fn inc(&mut self) -> Self {
        let prev = *self;
        self.value += 1;
        prev
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArgumentIndex {
    value: usize,
}

impl From<ArgumentIndex> for usize {
    fn from(value: ArgumentIndex) -> Self {
        value.value
    }
}

impl From<usize> for ArgumentIndex {
    fn from(value: usize) -> Self {
        ArgumentIndex { value }
    }
}

impl fmt::Display for ArgumentIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinOpOperator {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, PartialEq, Clone)]
pub enum IrInstruction {
    Mvi {
        dest: IrRegister,
        val: i64,
    },
    MvArg {
        dest: IrRegister,
        arg: ArgumentIndex,
    },

    BinOp {
        operator: BinOpOperator,
        dest: IrRegister,
        op1: IrRegister,
        op2: IrRegister,
    },
    Neg {
        dest: IrRegister,
        op: IrRegister,
    },

    Ret {
        reg: IrRegister,
    },
    Call {
        dest: IrRegister,
        name: String,
        function_id: FunctionId,
        args: Vec<IrRegister>,
    },
}

impl IrInstruction {
    pub fn operands(&self) -> impl Iterator<Item = IrRegister> {
        match self {
            IrInstruction::Mvi { dest, .. } => vec![*dest].into_iter(),
            IrInstruction::MvArg { dest, .. } => vec![*dest].into_iter(),
            IrInstruction::Neg { dest, op } => vec![*dest, *op].into_iter(),
            IrInstruction::BinOp {
                operator: _,
                dest,
                op1,
                op2,
            } => vec![*dest, *op1, *op2].into_iter(),
            IrInstruction::Ret { reg } => vec![*reg].into_iter(),
            IrInstruction::Call { dest, args, .. } => vec![*dest]
                .into_iter()
                .chain(args.iter().copied())
                .collect::<Vec<_>>()
                .into_iter(),
        }
    }
}

#[derive(Debug)]
pub struct CompiledFunction<'input> {
    pub name: &'input str,
    pub id: FunctionId,
    pub num_args: usize,
    pub body: Vec<IrInstruction>,
    pub num_used_registers: usize,
}

impl fmt::Display for IrInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IrInstruction::Mvi { dest, val } => write!(f, "mvi  @r{}, {}", dest, val),
            IrInstruction::MvArg { dest, arg } => write!(f, "mva  @r{}, a{}", dest, arg),
            IrInstruction::Neg { dest, op } => write!(f, "neg @r{}, r{}", dest, op),
            IrInstruction::BinOp {
                operator,
                dest,
                op1,
                op2,
            } => {
                write!(f, "{:?}  @r{}, r{}, r{}", operator, dest, op1, op2)
            }
            IrInstruction::Ret { reg } => write!(f, "ret  r{}", reg),
            IrInstruction::Call {
                dest,
                function_id,
                name,
                args,
            } => {
                write!(f, "call @r{}, {}:{}(", dest, name, function_id.0)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "r{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl fmt::Display for CompiledFunction<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "fn {} - #args: {}, #reg: {} {{",
            self.name, self.num_args, self.num_used_registers
        )?;
        for (i, instr) in self.body.iter().enumerate() {
            writeln!(f, "  {:-3}:  {}", i, instr)?;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
pub mod builders {
    use super::*;

    pub fn mvi(dest: u32, val: i64) -> IrInstruction {
        IrInstruction::Mvi {
            dest: IrRegister::from_u32(dest),
            val,
        }
    }

    pub fn mvarg(dest: u32, arg: usize) -> IrInstruction {
        IrInstruction::MvArg {
            dest: IrRegister::from_u32(dest),
            arg: ArgumentIndex::from(arg),
        }
    }

    pub fn neg(dest: u32, op: u32) -> IrInstruction {
        IrInstruction::Neg {
            dest: IrRegister::from_u32(dest),
            op: IrRegister::from_u32(op),
        }
    }

    pub fn add(dest: u32, op1: u32, op2: u32) -> IrInstruction {
        IrInstruction::BinOp {
            operator: BinOpOperator::Add,
            dest: IrRegister::from_u32(dest),
            op1: IrRegister::from_u32(op1),
            op2: IrRegister::from_u32(op2),
        }
    }

    pub fn sub(dest: u32, op1: u32, op2: u32) -> IrInstruction {
        IrInstruction::BinOp {
            operator: BinOpOperator::Sub,
            dest: IrRegister::from_u32(dest),
            op1: IrRegister::from_u32(op1),
            op2: IrRegister::from_u32(op2),
        }
    }

    pub fn mul(dest: u32, op1: u32, op2: u32) -> IrInstruction {
        IrInstruction::BinOp {
            operator: BinOpOperator::Mul,
            dest: IrRegister::from_u32(dest),
            op1: IrRegister::from_u32(op1),
            op2: IrRegister::from_u32(op2),
        }
    }

    pub fn div(dest: u32, op1: u32, op2: u32) -> IrInstruction {
        IrInstruction::BinOp {
            operator: BinOpOperator::Div,
            dest: IrRegister::from_u32(dest),
            op1: IrRegister::from_u32(op1),
            op2: IrRegister::from_u32(op2),
        }
    }

    pub fn ret(reg: u32) -> IrInstruction {
        IrInstruction::Ret {
            reg: IrRegister::from_u32(reg),
        }
    }

    pub fn call(dest: u32, name: &str, id: usize, args: Vec<u32>) -> IrInstruction {
        IrInstruction::Call {
            dest: IrRegister::from_u32(dest),
            function_id: FunctionId(id),
            name: name.to_string(),
            args: args.into_iter().map(IrRegister::from_u32).collect(),
        }
    }
}
