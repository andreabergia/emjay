use std::fmt::{Display, Write};

use crate::{
    backend::{BackendError, CompiledFunctionCatalog, GeneratedMachineCode, MachineCodeGenerator},
    backend_register_allocator::{self, AllocatedLocation},
    ir::{CompiledFunction, IrInstruction, IrRegister},
};

const NUM_SIZE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Register {
    Rax,
    Rcx,
    Rdx,
    Rbx,
    Rsp,
    Rbp,
    Rsi,
    R11,
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
            Register::R11 => 11,
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
            Register::R11 => write!(f, "r11"),
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
        value: i64,
    },
    MovRegToReg {
        source: Register,
        destination: Register,
    },
    AddRegToRax {
        register: Register,
    },
    SubRegFromRax {
        register: Register,
    },
    MulRegToRax {
        register: Register,
    },
    DivRegFromRax {
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
            X64Instruction::SubRegFromRax { register } => write!(f, "sub  rax, {}", register),
            X64Instruction::MulRegToRax { register } => write!(f, "add  rax, {}", register),
            X64Instruction::DivRegFromRax { register } => write!(f, "div  {}", register),
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
                vec.extend_from_slice(&(*value).to_le_bytes());
                vec
            }
            X64Instruction::MovRegToReg {
                source,
                destination,
            } => vec![0x48, 0x89, self.lookup_reg_reg(*source, *destination)?],
            X64Instruction::AddRegToRax { register } => {
                vec![0x48, 0x01, self.lookup_reg_reg(*register, Register::Rax)?]
            }
            X64Instruction::SubRegFromRax { register } => {
                vec![0x48, 0x29, self.lookup_reg_reg(*register, Register::Rax)?]
            }
            X64Instruction::MulRegToRax { register } => {
                vec![0x48, 0xF7, 0xE0 + register.index()]
            }
            X64Instruction::DivRegFromRax { register } => {
                vec![0x48, 0xF7, 0xF0 + register.index()]
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
            (Register::Rax, Register::Rsi) => Ok(0xC6),
            (Register::Rsi, Register::Rax) => Ok(0xF0),
            (Register::R11, Register::Rdx) => Ok(0xDA),
            (Register::Rdx, Register::R11) => Ok(0xD3),
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
        _function_catalog: &CompiledFunctionCatalog,
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
                IrInstruction::Mvi { dest, val } => {
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

                IrInstruction::Ret { reg } => {
                    self.move_to_accumulator(reg, &mut instructions)?;

                    // Epilogue and then return
                    instructions.push(X64Instruction::Pop {
                        register: Register::Rbp,
                    });
                    instructions.push(X64Instruction::Retn);
                }

                IrInstruction::Add { dest, op1, op2 } => self.do_bin_op(
                    &mut instructions,
                    op1,
                    op2,
                    dest,
                    |instructions, register| {
                        instructions.push(X64Instruction::AddRegToRax { register })
                    },
                )?,
                IrInstruction::Sub { dest, op1, op2 } => self.do_bin_op(
                    &mut instructions,
                    op1,
                    op2,
                    dest,
                    |instructions, register| {
                        instructions.push(X64Instruction::SubRegFromRax { register })
                    },
                )?,
                IrInstruction::Mul { dest, op1, op2 } => self.do_bin_op(
                    &mut instructions,
                    op1,
                    op2,
                    dest,
                    |instructions, register| {
                        instructions.push(X64Instruction::MulRegToRax { register })
                    },
                )?,
                IrInstruction::Div { dest, op1, op2 } => self.do_bin_op(
                    &mut instructions,
                    op1,
                    op2,
                    dest,
                    |instructions, register| {
                        // DIV is different from most other instructions: it will forcibly
                        // divide rdx:rax by the given register. For the accumulator we
                        // are fine, but we need to set rdx to zero, and to do so we backup
                        // its value. Furthermore, we might have that the divisor is actually
                        // in rdx. In that case, we move the divisor to r11 (which we know we
                        // have never allocated) and use `div r11`.
                        if register == Register::Rdx {
                            instructions.push(X64Instruction::MovRegToReg {
                                source: Register::Rdx,
                                destination: Register::R11,
                            });
                            instructions.push(X64Instruction::MovImmToReg {
                                register: Register::Rdx,
                                value: 0,
                            });
                            instructions.push(X64Instruction::DivRegFromRax {
                                register: Register::R11,
                            });
                            instructions.push(X64Instruction::MovRegToReg {
                                source: Register::R11,
                                destination: Register::Rdx,
                            });
                        } else {
                            instructions.push(X64Instruction::Push {
                                register: Register::Rdx,
                            });
                            instructions.push(X64Instruction::MovImmToReg {
                                register: Register::Rdx,
                                value: 0,
                            });
                            instructions.push(X64Instruction::DivRegFromRax { register });
                            instructions.push(X64Instruction::Pop {
                                register: Register::Rdx,
                            });
                        }
                    },
                )?,
                IrInstruction::MvArg { .. } => {
                    return Err(BackendError::NotImplemented(
                        "accessing function arguments".to_string(),
                    ))
                }
                IrInstruction::Call { .. } => {
                    return Err(BackendError::NotImplemented("function calls".to_string()))
                }

                IrInstruction::Neg { .. } => {
                    return Err(BackendError::NotImplemented("negate".to_string()))
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
        reg: &IrRegister,
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
        op1: &IrRegister,
        op2: &IrRegister,
        dest: &IrRegister,
        lambda: impl Fn(&mut Vec<X64Instruction>, Register),
    ) -> Result<(), BackendError> {
        self.move_to_accumulator(op1, instructions)?;

        let op2: usize = (*op2).into();
        match self.locations[op2] {
            AllocatedLocation::Stack { .. } => {
                return Err(BackendError::NotImplemented(
                    "binop when operand 2 is on the stack".to_string(),
                ))
            }
            AllocatedLocation::Register { register } => lambda(instructions, register),
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
    use trim_margin::MarginTrimmable;

    use super::*;
    use crate::{backend::CompiledFunctionCatalog, frontend, parser::*};

    #[test]
    fn can_compile_trivial_function() {
        let program = parse_program("fn the_answer() { return 1; }").unwrap();
        let compiled = frontend::compile(program).unwrap();
        assert_eq!(compiled.len(), 1);

        let mut gen = X64LinuxGenerator::default();
        let machine_code = gen
            .generate_machine_code(
                &compiled[0],
                &Box::new(CompiledFunctionCatalog::new(&compiled)),
            )
            .unwrap();
        assert_eq!(
            machine_code.machine_code,
            vec![
                0x55, 0x48, 0x89, 0xE5, 0x48, 0xB9, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x48, 0x89, 0xC8, 0x5D, 0xC3
            ]
        )
    }

    #[test]
    fn can_compile_math() {
        let program =
            parse_program("fn the_answer() { let a = 3; return a + 1 - 2 * 3 / 4; }").unwrap();
        let compiled = frontend::compile(program).unwrap();
        assert_eq!(compiled.len(), 1);

        let mut gen = X64LinuxGenerator::default();
        let machine_code = gen
            .generate_machine_code(
                &compiled[0],
                &Box::new(CompiledFunctionCatalog::new(&compiled)),
            )
            .unwrap();
        assert_eq!(
            "
            |push rbp
            |mov  rbp, rsp
            |mov  rcx, 3
            |mov  rdx, 1
            |mov  rax, rcx
            |add  rax, rdx
            |mov  rbx, rax
            |mov  rdx, 2
            |mov  rcx, 3
            |mov  rax, rdx
            |add  rax, rcx
            |mov  rsi, rax
            |mov  rdx, 4
            |mov  rax, rsi
            |mov  r11, rdx
            |mov  rdx, 0
            |div  r11
            |mov  rdx, r11
            |mov  rcx, rax
            |mov  rax, rbx
            |sub  rax, rcx
            |mov  rsi, rax
            |mov  rax, rsi
            |pop  rbp
            |retn
            |"
            .trim_margin()
            .unwrap(),
            machine_code.asm
        );
        assert_eq!(
            vec![
                0x55, 0x48, 0x89, 0xE5, 0x48, 0xB9, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x48, 0xBA, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x48, 0x89, 0xC8, 0x48,
                0x01, 0xD0, 0x48, 0x89, 0xC3, 0x48, 0xBA, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x48, 0xB9, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x48, 0x89, 0xD0,
                0x48, 0xF7, 0xE1, 0x48, 0x89, 0xC6, 0x48, 0xBA, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x48, 0x89, 0xF0, 0x48, 0x89, 0xD3, 0x48, 0xBA, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x48, 0xF7, 0xFB, 0x48, 0x89, 0xDA, 0x48, 0x89, 0xC1, 0x48,
                0x89, 0xD8, 0x48, 0x29, 0xC8, 0x48, 0x89, 0xC6, 0x48, 0x89, 0xF0, 0x5D, 0xC3
            ],
            machine_code.machine_code
        );
    }
}
