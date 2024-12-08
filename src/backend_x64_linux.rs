use std::{
    collections::HashMap,
    fmt::{Display, Write},
};

use crate::{
    backend::{GeneratedMachineCode, MachineCodeGenerator},
    ir::{CompiledFunction, RegisterIndex},
};

const NUM_SIZE: usize = 8;

#[derive(Debug, Clone, Copy)]
enum Register {
    Rax,
    Rcx,
    Rdx,
    Rbx,
    Rsp,
    Rbp,
    Rsi,
}

impl Register {
    fn index(&self) -> u8 {
        match self {
            Register::Rax => 0,
            Register::Rcx => 1,
            Register::Rdx => 2,
            Register::Rbx => 3,
            Register::Rsp => 4,
            Register::Rbp => 5,
            Register::Rsi => 6,
        }
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Register::Rax => write!(f, "rax"),
            Register::Rcx => write!(f, "rcx"),
            Register::Rdx => write!(f, "rdx"),
            Register::Rbx => write!(f, "rbx"),
            Register::Rsp => write!(f, "rsp"),
            Register::Rbp => write!(f, "rbp"),
            Register::Rsi => write!(f, "rsi"),
        }
    }
}

enum X64Instruction {
    Push {
        register: Register,
    },
    Pop {
        register: Register,
    },
    Retn,
    MovImmToReg {
        register: Register,
        value: f64,
    },
    MovRegToReg {
        source: Register,
        destination: Register,
    },
}

impl Display for X64Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            X64Instruction::Push { register: reg } => write!(f, "push {}", reg),
            X64Instruction::Pop { register: reg } => write!(f, "pop  {}", reg),
            X64Instruction::Retn => write!(f, "retn"),
            X64Instruction::MovImmToReg { register, value } => {
                write!(f, "mov  {}, {}", register, value)
            }
            X64Instruction::MovRegToReg {
                source,
                destination,
            } => write!(f, "mov  {}, {}", destination, source),
        }
    }
}

impl X64Instruction {
    fn into_machine_code(&self) -> Vec<u8> {
        match self {
            X64Instruction::Retn => vec![0xC3],
            X64Instruction::Push { register } => vec![0x50 + register.index()],
            X64Instruction::Pop { register } => vec![0x58 + register.index()],
            X64Instruction::MovImmToReg { register, value } => {
                let mut vec = vec![0xB8 + register.index()];
                vec.extend_from_slice(&(*value as i64).to_le_bytes());
                vec
            }
            X64Instruction::MovRegToReg {
                source,
                destination,
            } => vec![
                0x48,
                0x89,
                self.lookup_reg_reg(source.clone(), destination.clone()),
            ],
        }
    }

    fn lookup_reg_reg(&self, source: Register, destination: Register) -> u8 {
        match (source, destination) {
            (Register::Rax, Register::Rbx) => 0xC3,
            (Register::Rax, Register::Rcx) => 0xC1,
            (Register::Rax, Register::Rdx) => 0xC2,
            (Register::Rbx, Register::Rax) => 0xD8,
            (Register::Rcx, Register::Rax) => 0xC8,
            (Register::Rdx, Register::Rax) => 0xD0,

            (Register::Rsp, Register::Rbp) => 0xEC,
            (Register::Rbp, Register::Rsp) => 0xE5,
            _ => panic!("unimplemented {} -> {}", source, destination),
        }
    }
}

enum Location {
    Accumulator,
    Register { register: Register },
    Stack { offset: usize },
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Location::Accumulator => write!(f, "rax"),
            Location::Register { register: reg } => write!(f, "{}", reg),
            Location::Stack { offset } => write!(f, "rsp[{}]", offset),
        }
    }
}

#[derive(Default)]
pub struct X64LinuxGenerator {
    locations: HashMap<RegisterIndex, Location>,
}

impl MachineCodeGenerator for X64LinuxGenerator {
    fn generate_machine_code(&mut self, function: &CompiledFunction) -> GeneratedMachineCode {
        self.assign_locations(function);

        let mut instructions = Vec::new();

        instructions.push(X64Instruction::Push {
            register: Register::Rbp,
        });
        instructions.push(X64Instruction::MovRegToReg {
            source: Register::Rbp,
            destination: Register::Rsp,
        });

        for instruction in function.body.iter() {
            match instruction {
                crate::ir::Instruction::Mvi { dest, val } => {
                    let loc = self.locations.get(dest).unwrap();
                    match loc {
                        Location::Accumulator => todo!(),
                        Location::Stack { offset } => todo!(),
                        Location::Register { register } => {
                            instructions.push(X64Instruction::MovImmToReg {
                                register: *register,
                                value: *val,
                            })
                        }
                    }
                }

                crate::ir::Instruction::Ret { reg } => {
                    let loc = self.locations.get(reg).unwrap();
                    // Epilogue and then return
                    match loc {
                        Location::Accumulator => todo!(),
                        Location::Stack { offset } => todo!(),
                        Location::Register { register } => {
                            instructions.push(X64Instruction::MovRegToReg {
                                source: *register,
                                destination: Register::Rax,
                            })
                        }
                    }
                    instructions.push(X64Instruction::Pop {
                        register: Register::Rbp,
                    });
                    instructions.push(X64Instruction::Retn);
                }

                //crate::ir::Instruction::Add { dest, op1, op2 } => {
                //    let loc_dest = self.locations.get(dest).unwrap();
                //    let loc1 = self.locations.get(op1).unwrap();
                //    let loc2 = self.locations.get(op2).unwrap();
                //    writeln!(&mut asm, "mov rax, {loc1}");
                //    writeln!(&mut asm, "add rax, {loc2}");
                //    writeln!(&mut asm, "mov {loc_dest}, rax");
                //}
                _ => todo!(),
            }
        }

        let mut asm = String::new();
        let mut machine_code: Vec<u8> = Vec::new();

        for instruction in instructions {
            writeln!(&mut asm, "{}", instruction);
            machine_code.extend(instruction.into_machine_code());
        }

        GeneratedMachineCode { asm, machine_code }
    }
}

impl X64LinuxGenerator {
    fn assign_locations(&mut self, function: &CompiledFunction) {
        for i in 0..function.max_used_registers.into() {
            match i {
                0 => self.locations.insert(
                    RegisterIndex::from(i),
                    Location::Register {
                        register: Register::Rcx,
                    },
                ),
                1 => self.locations.insert(
                    RegisterIndex::from(i),
                    Location::Register {
                        register: Register::Rdx,
                    },
                ),
                2 => self.locations.insert(
                    RegisterIndex::from(i),
                    Location::Register {
                        register: Register::Rbx,
                    },
                ),
                3 => self.locations.insert(
                    RegisterIndex::from(i),
                    Location::Register {
                        register: Register::Rsi,
                    },
                ),
                n => self.locations.insert(
                    RegisterIndex::from(i),
                    Location::Stack {
                        offset: ((n as usize) - 3) * NUM_SIZE,
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
        let program = parse_program("fn the_answer() { let a = 42; return a; }").unwrap();
        let compiled = frontend::compile(program);
        assert_eq!(compiled.len(), 1);

        let mut gen = X64LinuxGenerator::default();
        let machine_code = gen.generate_machine_code(&compiled[0]);
        println!("{}", machine_code.asm);
        machine_code
            .machine_code
            .iter()
            .for_each(|byte| print!("{:02X} ", byte));
    }
}
