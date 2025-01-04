use std::fmt::{Display, Write};

use crate::{
    backend::{BackendError, GeneratedMachineCode, MachineCodeGenerator},
    backend_register_allocator::{self, AllocatedLocation},
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
    AddRegToRax {
        register: Register,
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
            X64Instruction::AddRegToRax { register } => write!(f, "add  rax, {}", register),
        }
    }
}

impl X64Instruction {
    fn make_machine_code(&self) -> Result<Vec<u8>, BackendError> {
        Ok(match self {
            X64Instruction::Retn => vec![0xC3],
            X64Instruction::Push { register } => vec![0x50 + register.index()],
            X64Instruction::Pop { register } => vec![0x58 + register.index()],
            X64Instruction::MovImmToReg { register, value } => {
                let mut vec = vec![0x48, 0xB8 + register.index()];
                println!("vec : {:?}", vec);
                vec.extend_from_slice(&(*value as i64).to_le_bytes());
                println!("vec : {:?}", vec);
                vec
            }
            X64Instruction::MovRegToReg {
                source,
                destination,
            } => vec![0x48, 0x89, self.lookup_reg_reg(*source, *destination)?],
            X64Instruction::AddRegToRax { register } => {
                vec![0x48, 0x01, self.lookup_reg_reg(*register, Register::Rax)?]
            }
        })
    }

    // TODO: I am not clear how to encode this in a generalized way.
    fn lookup_reg_reg(&self, source: Register, destination: Register) -> Result<u8, BackendError> {
        match (source, destination) {
            (Register::Rax, Register::Rbx) => Ok(0xC3),
            (Register::Rax, Register::Rcx) => Ok(0xC1),
            (Register::Rax, Register::Rdx) => Ok(0xC2),
            (Register::Rbx, Register::Rax) => Ok(0xD8),
            (Register::Rcx, Register::Rax) => Ok(0xC8),
            (Register::Rdx, Register::Rax) => Ok(0xD0),

            (Register::Rsp, Register::Rbp) => Ok(0xE5),
            (Register::Rbp, Register::Rsp) => Ok(0xEC),
            _ => Err(BackendError::NotImplemented(format!(
                "encoding of move from reg {source} to reg {destination}",
            ))),
        }
    }
}

#[derive(Default)]
pub struct X64LinuxGenerator {
    locations: Vec<AllocatedLocation<Register>>,
}

impl MachineCodeGenerator for X64LinuxGenerator {
    fn generate_machine_code(
        &mut self,
        function: &CompiledFunction,
    ) -> Result<GeneratedMachineCode, BackendError> {
        self.allocate_registers(function);

        let mut instructions = Vec::new();

        instructions.push(X64Instruction::Push {
            register: Register::Rbp,
        });
        instructions.push(X64Instruction::MovRegToReg {
            source: Register::Rsp,
            destination: Register::Rbp,
        });

        for instruction in function.body.iter() {
            match instruction {
                crate::ir::Instruction::Mvi { dest, val } => {
                    let dest: usize = (*dest).into();
                    match self.locations[dest] {
                        AllocatedLocation::Stack { .. } => {
                            return Err(BackendError::NotImplemented(
                                "move immediate to stack".to_string(),
                            ))
                        }
                        AllocatedLocation::Register { register } => {
                            instructions.push(X64Instruction::MovImmToReg {
                                register,
                                value: *val,
                            })
                        }
                    }
                }

                crate::ir::Instruction::Ret { reg } => {
                    self.move_to_accumulator(reg, &mut instructions)?;

                    // Epilogue and then return
                    instructions.push(X64Instruction::Pop {
                        register: Register::Rbp,
                    });
                    instructions.push(X64Instruction::Retn);
                }

                crate::ir::Instruction::Add { dest, op1, op2 } => {
                    self.do_bin_op(&mut instructions, op1, op2, dest, |register| {
                        X64Instruction::AddRegToRax { register }
                    })?
                }

                _ => {
                    return Err(BackendError::NotImplemented(format!(
                        "instruction {:?}",
                        instruction
                    )))
                }
            }
        }

        let mut asm = String::new();
        let mut machine_code: Vec<u8> = Vec::new();

        for instruction in instructions {
            let _ = writeln!(&mut asm, "{}", instruction);
            machine_code.extend(instruction.make_machine_code()?);
        }

        Ok(GeneratedMachineCode { asm, machine_code })
    }
}

impl X64LinuxGenerator {
    fn allocate_registers(&mut self, function: &CompiledFunction) {
        let allocations = backend_register_allocator::allocate(
            function,
            vec![Register::Rcx, Register::Rdx, Register::Rbx, Register::Rsi],
        );
        self.locations.extend(allocations);
    }

    fn move_to_accumulator(
        &mut self,
        reg: &RegisterIndex,
        instructions: &mut Vec<X64Instruction>,
    ) -> Result<(), BackendError> {
        let reg: usize = (*reg).into();
        match self.locations[reg] {
            AllocatedLocation::Register { register } => {
                instructions.push(X64Instruction::MovRegToReg {
                    source: register,
                    destination: Register::Rax,
                });
                Ok(())
            }
            AllocatedLocation::Stack { .. } => Err(BackendError::NotImplemented(
                "move to accumulator from stack".to_string(),
            )),
        }
    }

    fn do_bin_op(
        &mut self,
        instructions: &mut Vec<X64Instruction>,
        op1: &RegisterIndex,
        op2: &RegisterIndex,
        dest: &RegisterIndex,
        lambda: impl Fn(Register) -> X64Instruction,
    ) -> Result<(), BackendError> {
        self.move_to_accumulator(op1, instructions)?;

        let op2: usize = (*op2).into();
        match self.locations[op2] {
            AllocatedLocation::Stack { .. } => {
                return Err(BackendError::NotImplemented(
                    "binop when operand 2 is on the stack".to_string(),
                ))
            }
            AllocatedLocation::Register { register } => instructions.push(lambda(register)),
        }

        let dest: usize = (*dest).into();
        match self.locations[dest] {
            AllocatedLocation::Register { register } => {
                instructions.push(X64Instruction::MovRegToReg {
                    source: Register::Rax,
                    destination: register,
                });
                Ok(())
            }
            AllocatedLocation::Stack { .. } => Err(BackendError::NotImplemented(
                "binop when destination is on the stack".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{frontend, parser::*};

    #[test]
    fn can_compile_trivial_function() {
        let program = parse_program("fn the_answer() { let a = 42; return a + 1; }").unwrap();
        let compiled = frontend::compile(program).unwrap();
        assert_eq!(compiled.len(), 1);

        let mut gen = X64LinuxGenerator::default();
        let machine_code = gen.generate_machine_code(&compiled[0]).unwrap();
        assert_eq!(
            machine_code.machine_code,
            vec![
                0x55, 0x48, 0x89, 0xE5, 0x48, 0xB9, 0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x48, 0xBA, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x48, 0x89, 0xC8, 0x48,
                0x01, 0xD0, 0x48, 0x89, 0xC3, 0x48, 0x89, 0xD8, 0x5D, 0xC3
            ]
        )
    }
}
