use std::{
    collections::HashMap,
    fmt::{Display, Write},
};

use crate::ir::{CompiledFunction, RegisterIndex};

const NUM_SIZE: usize = 8;

enum Location {
    Accumulator,
    Register { reg: GeneralPurposeRegister },
    Stack { offset: usize },
}

enum GeneralPurposeRegister {
    Rbx,
    Rcx,
    Rdx,
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Location::Accumulator => write!(f, "rax"),
            Location::Register { reg } => write!(f, "{}", reg),
            Location::Stack { offset } => write!(f, "rsp[{}]", offset),
        }
    }
}

impl Display for GeneralPurposeRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneralPurposeRegister::Rbx => write!(f, "rbx"),
            GeneralPurposeRegister::Rcx => write!(f, "rcx"),
            GeneralPurposeRegister::Rdx => write!(f, "rdx"),
        }
    }
}

#[derive(Default)]
struct X64LinuxGenerator {
    locations: HashMap<RegisterIndex, Location>,
}

impl X64LinuxGenerator {
    pub fn generate_machine_code(&mut self, function: &CompiledFunction) -> String {
        self.assign_locations(function);

        let mut asm = String::new();

        writeln!(&mut asm, "push rbp");
        writeln!(&mut asm, "mov rbp, rsp");

        for instruction in function.body.iter() {
            match instruction {
                crate::ir::Instruction::Mvi { dest, val } => {
                    let loc = self.locations.get(dest).unwrap();
                    writeln!(&mut asm, "movi {loc}, {val}");
                }

                crate::ir::Instruction::Ret { reg } => {
                    let loc = self.locations.get(reg).unwrap();
                    writeln!(&mut asm, "mov rax, {loc}");
                    writeln!(&mut asm, "pop rbp");
                    writeln!(&mut asm, "ret");
                }

                crate::ir::Instruction::Add { dest, op1, op2 } => {
                    let loc_dest = self.locations.get(dest).unwrap();
                    let loc1 = self.locations.get(op1).unwrap();
                    let loc2 = self.locations.get(op2).unwrap();
                    writeln!(&mut asm, "mov rax, {loc1}");
                    writeln!(&mut asm, "add rax, {loc2}");
                    writeln!(&mut asm, "mov {loc_dest}, rax");
                }
                crate::ir::Instruction::Sub { dest, op1, op2 }
                | crate::ir::Instruction::Mul { dest, op1, op2 }
                | crate::ir::Instruction::Div { dest, op1, op2 } => {
                    println!("todo");
                    todo!()
                }
            }
        }

        asm
    }

    fn assign_locations(&mut self, function: &CompiledFunction) {
        for i in 0..function.max_used_registers.into() {
            match i {
                0 => self.locations.insert(
                    RegisterIndex::from(i),
                    Location::Register {
                        reg: GeneralPurposeRegister::Rbx,
                    },
                ),
                1 => self.locations.insert(
                    RegisterIndex::from(i),
                    Location::Register {
                        reg: GeneralPurposeRegister::Rcx,
                    },
                ),
                2 => self.locations.insert(
                    RegisterIndex::from(i),
                    Location::Register {
                        reg: GeneralPurposeRegister::Rdx,
                    },
                ),
                n => self.locations.insert(
                    RegisterIndex::from(i),
                    Location::Stack {
                        offset: ((n as usize) - 2) * NUM_SIZE,
                    },
                ),
            };
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{frontend, parser::*};

    #[test]
    fn can_compile_trivial_function() {
        let program = parse_program("fn the_answer() { return 1 + 2; }").unwrap();
        let compiled = frontend::compile(program);
        assert_eq!(compiled.len(), 1);

        let mut gen = X64LinuxGenerator::default();
        let machine_code = gen.generate_machine_code(&compiled[0]);
        println!("{machine_code}");
    }
}
