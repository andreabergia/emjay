use std::fmt::{Display, Write};

use crate::{
    backend::{BackendError, CompiledFunctionCatalog, GeneratedMachineCode, MachineCodeGenerator},
    backend_register_allocator::{self, AllocatedLocation},
    ir::{CompiledFunction, Instruction, RegisterIndex},
    jit::jit_call_trampoline,
};

#[derive(Debug, Clone, Copy)]
enum Register {
    X0,
    X1,
    X2,
    X3,
    X4,
    X5,
    X6,
    X7,
    X8,
    X9,
    X10,
    X11,
    X12,
    X13,
    X14,
    X15,
    X16,
    X17,
    X18,
    X19,
    X20,
    X21,
    X22,
    X23,
    X24,
    X25,
    X26,
    X27,
    X28,
    X29,
    X30,
    Sp,
}

impl Register {
    fn index(&self) -> u32 {
        match self {
            Register::X0 => 0,
            Register::X1 => 1,
            Register::X2 => 2,
            Register::X3 => 3,
            Register::X4 => 4,
            Register::X5 => 5,
            Register::X6 => 6,
            Register::X7 => 7,
            Register::X8 => 8,
            Register::X9 => 9,
            Register::X10 => 10,
            Register::X11 => 11,
            Register::X12 => 12,
            Register::X13 => 13,
            Register::X14 => 14,
            Register::X15 => 15,
            Register::X16 => 16,
            Register::X17 => 17,
            Register::X18 => 18,
            Register::X19 => 19,
            Register::X20 => 20,
            Register::X21 => 21,
            Register::X22 => 22,
            Register::X23 => 23,
            Register::X24 => 24,
            Register::X25 => 25,
            Register::X26 => 26,
            Register::X27 => 27,
            Register::X28 => 28,
            Register::X29 => 29,
            Register::X30 => 30,
            Register::Sp => 31,
        }
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Register::X0 => write!(f, "x0"),
            Register::X1 => write!(f, "x1"),
            Register::X2 => write!(f, "x2"),
            Register::X3 => write!(f, "x3"),
            Register::X4 => write!(f, "x4"),
            Register::X5 => write!(f, "x5"),
            Register::X6 => write!(f, "x6"),
            Register::X7 => write!(f, "x7"),
            Register::X8 => write!(f, "x8"),
            Register::X9 => write!(f, "x9"),
            Register::X10 => write!(f, "x10"),
            Register::X11 => write!(f, "x11"),
            Register::X12 => write!(f, "x12"),
            Register::X13 => write!(f, "x13"),
            Register::X14 => write!(f, "x14"),
            Register::X15 => write!(f, "x15"),
            Register::X16 => write!(f, "x16"),
            Register::X17 => write!(f, "x17"),
            Register::X18 => write!(f, "x18"),
            Register::X19 => write!(f, "x19"),
            Register::X20 => write!(f, "x20"),
            Register::X21 => write!(f, "x21"),
            Register::X22 => write!(f, "x22"),
            Register::X23 => write!(f, "x23"),
            Register::X24 => write!(f, "x24"),
            Register::X25 => write!(f, "x25"),
            Register::X26 => write!(f, "x26"),
            Register::X27 => write!(f, "x27"),
            Register::X28 => write!(f, "x28"),
            Register::X29 => write!(f, "x29"),
            Register::X30 => write!(f, "x30"),
            Register::Sp => write!(f, "sp"),
        }
    }
}

enum Aarch64Instruction {
    Nop,
    Ret,
    MovImmToReg {
        register: Register,
        value: f64,
    },
    MovRegToReg {
        source: Register,
        destination: Register,
    },
    MovSpToReg {
        destination: Register,
    },
    AddRegToReg {
        destination: Register,
        reg1: Register,
        reg2: Register,
    },
    SubRegToReg {
        destination: Register,
        reg1: Register,
        reg2: Register,
    },
    MulRegToReg {
        destination: Register,
        reg1: Register,
        reg2: Register,
    },
    DivRegToReg {
        destination: Register,
        reg1: Register,
        reg2: Register,
    },
    Blr {
        register: Register,
    },
    Str {
        source: Register,
        base: Register,
        offset: u32,
    },
    Ldr {
        destination: Register,
        base: Register,
        offset: u32,
    },
    Stp {
        reg1: Register,
        reg2: Register,
        base: Register,
        offset: i32,
        pre_indexing: bool,
    },
    Ldp {
        reg1: Register,
        reg2: Register,
        base: Register,
        offset: i32,
    },
}

impl Display for Aarch64Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Aarch64Instruction::Nop => write!(f, "nop"),
            Aarch64Instruction::Ret => write!(f, "ret"),
            Aarch64Instruction::MovImmToReg { register, value } => {
                write!(f, "movz {}, {}", register, value)
            }
            Aarch64Instruction::MovRegToReg {
                source,
                destination,
            } => {
                write!(f, "mov  {}, {}", destination, source)
            }
            Aarch64Instruction::MovSpToReg { destination } => {
                write!(f, "mov  {}, sp", destination)
            }
            Aarch64Instruction::AddRegToReg {
                destination,
                reg1,
                reg2,
            } => write!(f, "add  {}, {}, {}", destination, reg1, reg2),
            Aarch64Instruction::SubRegToReg {
                destination,
                reg1,
                reg2,
            } => write!(f, "subs {}, {}, {}", destination, reg1, reg2),
            Aarch64Instruction::MulRegToReg {
                destination,
                reg1,
                reg2,
            } => write!(f, "mul  {}, {}, {}", destination, reg1, reg2),
            Aarch64Instruction::DivRegToReg {
                destination,
                reg1,
                reg2,
            } => write!(f, "sdiv {}, {}, {}", destination, reg1, reg2),
            Aarch64Instruction::Blr { register } => write!(f, "blr {}", register),
            Aarch64Instruction::Str {
                source,
                base,
                offset,
            } => write!(f, "str  {}, [{}, #{}]", source, base, offset),
            Aarch64Instruction::Ldr {
                destination,
                base,
                offset,
            } => write!(f, "ldr  {}, [{}, #{}]", destination, base, offset),
            Aarch64Instruction::Stp {
                reg1,
                reg2,
                base,
                offset,
                pre_indexing,
            } => {
                let pre_indexing = if *pre_indexing { "!" } else { "" };
                write!(
                    f,
                    "stp  {}, {}, [{}, #{}]{}",
                    reg1, reg2, base, offset, pre_indexing
                )
            }
            Aarch64Instruction::Ldp {
                reg1,
                reg2,
                base,
                offset,
            } => write!(f, "ldp  {}, {}, [{}], #{}", reg1, reg2, base, offset),
        }
    }
}

impl Aarch64Instruction {
    const MOVZ: u32 = 0xD2800000;
    const MOVK_SHIFT_16: u32 = 0xF2A00000;
    const MOVK_SHIFT_32: u32 = 0xF2C00000;
    const MOVK_SHIFT_48: u32 = 0xF2E00000;
    const MOV: u32 = 0xAA0003E0;
    const MOV_SP_TO_REG: u32 = 0x910003e0;
    const ADD: u32 = 0x8B000000;
    const SUBS: u32 = 0xEB000000;
    const MUL: u32 = 0x9B007C00;
    const SDIV: u32 = 0x9AC00C00;
    const BLR: u32 = 0xD63F0000;
    const STR: u32 = 0xF9000000;
    const LDR: u32 = 0xF9400000;
    const STP: u32 = 0xA9000000;
    const STP_PRE_INDEX: u32 = 0xA9800000;
    const LDP: u32 = 0xA8C00000;

    fn make_machine_code(&self) -> Vec<u8> {
        match self {
            Aarch64Instruction::Nop => vec![0xD5, 0x03, 0x20, 0x1F],
            Aarch64Instruction::Ret => vec![0xC0, 0x03, 0x5F, 0xD6],

            Aarch64Instruction::MovImmToReg { register, value } => {
                // Note: there are a lot more efficient encoding: for example, we always
                // use 64 bit registers here, and we could use the bitmask immediate
                // trick described here:
                // https://kddnewton.com/2022/08/11/aarch64-bitmask-immediates.html
                // But, since this is a toy, I don't really care about efficiency. :-)

                let mut result: Vec<u8> = Vec::with_capacity(8);
                let imm = *value as u64;

                result.extend(Self::mov_imm(Self::MOVZ, imm & 0xFFFF, *register));
                if imm > 0xFFFF {
                    result.extend(Self::mov_imm(
                        Self::MOVK_SHIFT_16,
                        (imm >> 16) & 0xFFFF,
                        *register,
                    ));
                }
                if imm > 0xFFFFFFFF {
                    result.extend(Self::mov_imm(
                        Self::MOVK_SHIFT_32,
                        (imm >> 32) & 0xFFFF,
                        *register,
                    ));
                }
                if imm > 0xFFFFFFFFFFFF {
                    result.extend(Self::mov_imm(
                        Self::MOVK_SHIFT_48,
                        (imm >> 48) & 0xFFFF,
                        *register,
                    ));
                }

                result
            }

            Aarch64Instruction::MovRegToReg {
                source,
                destination,
            } => {
                let mut i: u32 = Self::MOV;
                i |= source.index() << 16;
                i |= destination.index();
                i.to_le_bytes().to_vec()
            }

            Aarch64Instruction::MovSpToReg { destination } => {
                let mut i: u32 = Self::MOV_SP_TO_REG;
                i |= destination.index();
                i.to_le_bytes().to_vec()
            }

            Aarch64Instruction::AddRegToReg {
                destination,
                reg1,
                reg2,
            } => Self::encode_three_reg_op(Self::ADD, destination, reg1, reg2),

            Aarch64Instruction::SubRegToReg {
                destination,
                reg1,
                reg2,
            } => Self::encode_three_reg_op(Self::SUBS, destination, reg1, reg2),

            Aarch64Instruction::MulRegToReg {
                destination,
                reg1,
                reg2,
            } => Self::encode_three_reg_op(Self::MUL, destination, reg1, reg2),

            Aarch64Instruction::DivRegToReg {
                destination,
                reg1,
                reg2,
            } => Self::encode_three_reg_op(Self::SDIV, destination, reg1, reg2),

            Aarch64Instruction::Blr { register } => {
                let mut i = Self::BLR;
                i |= register.index() << 5;
                i.to_le_bytes().to_vec()
            }

            Aarch64Instruction::Str {
                source,
                base,
                offset,
            } => {
                let mut i = Self::STR;
                i |= base.index() << 5;
                i |= source.index();
                i |= (offset >> 3) << 10;
                i.to_le_bytes().to_vec()
            }

            Aarch64Instruction::Ldr {
                destination,
                base,
                offset,
            } => {
                let mut i = Self::LDR;
                i |= base.index() << 5;
                i |= destination.index();
                i |= (offset >> 3) << 10;
                i.to_le_bytes().to_vec()
            }

            Aarch64Instruction::Stp {
                reg1,
                reg2,
                base,
                offset,
                pre_indexing,
            } => {
                let mut i = if *pre_indexing {
                    Self::STP_PRE_INDEX
                } else {
                    Self::STP
                };
                i |= reg1.index();
                i |= reg2.index() << 10;
                i |= base.index() << 5;
                let offset: u32 = unsafe {
                    std::mem::transmute(if *offset > 0 {
                        offset >> 3
                    } else {
                        (offset >> 3) & 0x7F
                    })
                };
                i |= offset << 15;
                i.to_le_bytes().to_vec()
            }

            Aarch64Instruction::Ldp {
                reg1,
                reg2,
                base,
                offset,
            } => {
                let mut i = Self::LDP;
                i |= reg1.index();
                i |= reg2.index() << 10;
                i |= base.index() << 5;
                let offset: u32 = unsafe {
                    std::mem::transmute(if *offset > 0 {
                        offset >> 3
                    } else {
                        (offset >> 3) & 0x7F
                    })
                };
                i |= offset << 15;
                i.to_le_bytes().to_vec()
            }
        }
    }

    fn mov_imm(base: u32, immediate: u64, register: Register) -> Vec<u8> {
        let mut i0 = base;
        i0 |= ((immediate & 0xFFFF) as u32) << 5;
        i0 |= register.index();
        i0.to_le_bytes().to_vec()
    }

    fn encode_three_reg_op(
        base: u32,
        destination: &Register,
        reg1: &Register,
        reg2: &Register,
    ) -> Vec<u8> {
        let mut i: u32 = base;
        i |= reg1.index() << 5;
        i |= reg2.index() << 16;
        i |= destination.index();
        i.to_le_bytes().to_vec()
    }
}

#[derive(Default)]
pub struct Aarch64Generator {
    locations: Vec<AllocatedLocation<Register>>,
    stack_offset: u32,
}

impl MachineCodeGenerator for Aarch64Generator {
    fn generate_machine_code(
        &mut self,
        function: &CompiledFunction,
        function_catalog: &Box<CompiledFunctionCatalog>,
    ) -> Result<GeneratedMachineCode, BackendError> {
        self.allocate_registers(function);

        let mut instructions = Vec::new();
        instructions.push(Aarch64Instruction::Stp {
            reg1: Register::X29,
            reg2: Register::X30,
            base: Register::Sp,
            offset: -32,
            pre_indexing: true,
        });
        instructions.push(Aarch64Instruction::MovSpToReg {
            destination: Register::X29,
        });

        for instruction in function.body.iter() {
            match instruction {
                Instruction::Mvi { dest, val } => {
                    let dest: usize = (*dest).into();
                    match self.locations[dest] {
                        AllocatedLocation::Register { register } => {
                            instructions.push(Aarch64Instruction::MovImmToReg {
                                register,
                                value: *val,
                            })
                        }
                        AllocatedLocation::Stack { offset: _ } => {
                            return Err(BackendError::NotImplemented(
                                "move immediate to stack".to_string(),
                            ))
                        }
                    }
                }

                Instruction::Ret { reg } => {
                    let dest: usize = (*reg).into();
                    match self.locations[dest] {
                        AllocatedLocation::Register { register } => {
                            instructions.push(Aarch64Instruction::MovRegToReg {
                                source: register,
                                destination: Register::X0,
                            });
                        }
                        AllocatedLocation::Stack { offset: _ } => {
                            return Err(BackendError::NotImplemented(
                                "return value from stack".to_string(),
                            ))
                        }
                    }
                    instructions.push(Aarch64Instruction::Ldp {
                        reg1: Register::X29,
                        reg2: Register::X30,
                        base: Register::Sp,
                        offset: 32,
                    });
                    instructions.push(Aarch64Instruction::Ret);
                }

                Instruction::Add { dest, op1, op2 } => {
                    self.do_binop(
                        &mut instructions,
                        *dest,
                        *op1,
                        *op2,
                        |destination, reg1, reg2| Aarch64Instruction::AddRegToReg {
                            destination,
                            reg1,
                            reg2,
                        },
                    )?;
                }

                Instruction::Sub { dest, op1, op2 } => {
                    self.do_binop(
                        &mut instructions,
                        *dest,
                        *op1,
                        *op2,
                        |destination, reg1, reg2| Aarch64Instruction::SubRegToReg {
                            destination,
                            reg1,
                            reg2,
                        },
                    )?;
                }

                Instruction::Mul { dest, op1, op2 } => {
                    self.do_binop(
                        &mut instructions,
                        *dest,
                        *op1,
                        *op2,
                        |destination, reg1, reg2| Aarch64Instruction::MulRegToReg {
                            destination,
                            reg1,
                            reg2,
                        },
                    )?;
                }

                Instruction::Div { dest, op1, op2 } => {
                    self.do_binop(
                        &mut instructions,
                        *dest,
                        *op1,
                        *op2,
                        |destination, reg1, reg2| Aarch64Instruction::DivRegToReg {
                            destination,
                            reg1,
                            reg2,
                        },
                    )?;
                }

                Instruction::Call { dest, name } => {
                    let dest: usize = (*dest).into();

                    let called_function_index = function_catalog
                        .get_function_id(name)
                        .ok_or_else(|| BackendError::FunctionNotFound(name.to_string()))?;

                    let fn_catalog_addr: usize =
                        unsafe { std::mem::transmute(&**function_catalog) };
                    let jit_call_trampoline_address: usize =
                        (jit_call_trampoline as fn(_, _) -> _) as usize;

                    // Leave space to move the address as an immediate to register X19
                    //self.push(&mut instructions, Register::X19);
                    //self.push(&mut instructions, Register::X1);

                    instructions.push(Aarch64Instruction::MovImmToReg {
                        register: Register::X0,
                        value: fn_catalog_addr as f64,
                    });
                    instructions.push(Aarch64Instruction::MovImmToReg {
                        register: Register::X1,
                        value: called_function_index.0 as f64,
                    });
                    instructions.push(Aarch64Instruction::MovImmToReg {
                        register: Register::X19,
                        value: jit_call_trampoline_address as f64,
                    });
                    instructions.push(Aarch64Instruction::Blr {
                        register: Register::X19,
                    });

                    match self.locations[dest] {
                        AllocatedLocation::Register {
                            register: destination,
                        } => instructions.push(Aarch64Instruction::MovRegToReg {
                            source: Register::X0,
                            destination,
                        }),
                        AllocatedLocation::Stack { offset: _ } => {
                            return Err(BackendError::NotImplemented(
                                "move register to stack".to_string(),
                            ))
                        }
                    }
                    // TODO: enable
                    //self.pop(&mut instructions, Register::X1);
                    //self.pop(&mut instructions, Register::X19);
                }
            }
        }

        let mut asm = String::new();
        let mut machine_code: Vec<u8> = Vec::new();

        for instruction in instructions {
            let _ = writeln!(&mut asm, "{}", instruction);
            machine_code.extend(instruction.make_machine_code());
        }

        Ok(GeneratedMachineCode { asm, machine_code })
    }
}

impl Aarch64Generator {
    fn allocate_registers(&mut self, function: &CompiledFunction) {
        let allocations = backend_register_allocator::allocate::<Register>(
            function,
            vec![
                Register::X8,
                Register::X9,
                Register::X10,
                Register::X11,
                Register::X12,
                Register::X13,
                Register::X14,
                Register::X15,
            ],
        );
        self.locations.extend(allocations);
    }

    fn do_binop(
        &self,
        instructions: &mut Vec<Aarch64Instruction>,
        dest: RegisterIndex,
        op1: RegisterIndex,
        op2: RegisterIndex,
        callback: impl Fn(Register, Register, Register) -> Aarch64Instruction,
    ) -> Result<(), BackendError> {
        let op1: usize = op1.into();
        let op2: usize = op2.into();
        let dest: usize = dest.into();

        match self.locations[op1] {
            AllocatedLocation::Register { register: reg1 } => match self.locations[op2] {
                AllocatedLocation::Register { register: reg2 } => match self.locations[dest] {
                    AllocatedLocation::Register { register: dest } => {
                        instructions.push(callback(dest, reg1, reg2));
                        Ok(())
                    }
                    AllocatedLocation::Stack { offset: _ } => Err(BackendError::NotImplemented(
                        "binop when destination is in stack".to_string(),
                    )),
                },
                AllocatedLocation::Stack { offset: _ } => Err(BackendError::NotImplemented(
                    "binop when one operand is in stack".to_string(),
                )),
            },
            AllocatedLocation::Stack { offset: _ } => Err(BackendError::NotImplemented(
                "binop when one operand is in stack".to_string(),
            )),
        }
    }

    fn push(&mut self, instructions: &mut Vec<Aarch64Instruction>, register: Register) {
        self.stack_offset += 8;
        instructions.push(Aarch64Instruction::Str {
            source: register,
            base: Register::X29,
            offset: self.stack_offset,
        });
    }

    fn pop(&mut self, instructions: &mut Vec<Aarch64Instruction>, register: Register) {
        self.stack_offset -= 8;
        instructions.push(Aarch64Instruction::Ldr {
            destination: register,
            base: Register::X29,
            offset: self.stack_offset,
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{backend::CompiledFunctionCatalog, frontend, parser::*};
    use proptest::prelude::*;
    use trim_margin::MarginTrimmable;

    fn assert_encodes_as(instruction: Aarch64Instruction, expected_machine_code: Vec<u8>) {
        let machine_code = instruction.make_machine_code();
        assert_eq!(expected_machine_code, machine_code);
    }

    #[test]
    fn can_encode_move_immediate_16_bit() {
        assert_encodes_as(
            Aarch64Instruction::MovImmToReg {
                register: Register::X1,
                value: 123.,
            },
            vec![0x61, 0x0F, 0x80, 0xD2],
        );
    }

    #[test]
    fn can_encode_move_immediate_32_bit() {
        assert_encodes_as(
            Aarch64Instruction::MovImmToReg {
                register: Register::X1,
                value: 1234567.,
            },
            vec![0xE1, 0xD0, 0x9A, 0xD2, 0x41, 0x02, 0xA0, 0xF2],
        );
    }

    #[test]
    fn can_encode_move_immediate_48_bit() {
        assert_encodes_as(
            Aarch64Instruction::MovImmToReg {
                register: Register::X1,
                value: 12345678901.,
            },
            vec![
                0xA1, 0x86, 0x83, 0xD2, 0x81, 0xFB, 0xBB, 0xF2, 0x41, 0x00, 0xC0, 0xF2,
            ],
        );
    }

    #[test]
    fn can_encode_move_immediate_64_bit() {
        assert_encodes_as(
            Aarch64Instruction::MovImmToReg {
                register: Register::X1,
                value: 1234567890123456.,
            },
            vec![
                0x01, 0x58, 0x97, 0xd2, 0x41, 0x91, 0xA7, 0xF2, 0xA1, 0x5A, 0xCC, 0xF2, 0x81, 0x00,
                0xE0, 0xF2,
            ],
        );
    }

    #[test]
    fn can_encode_move_reg_to_reg() {
        assert_encodes_as(
            Aarch64Instruction::MovRegToReg {
                source: Register::X8,
                destination: Register::X9,
            },
            vec![0xE9, 0x03, 0x08, 0xAA],
        );
    }

    #[test]
    fn can_encode_mov_sp_to_reg() {
        assert_encodes_as(
            Aarch64Instruction::MovSpToReg {
                destination: Register::X29,
            },
            vec![0xFD, 0x03, 0x00, 0x91],
        );
    }

    #[test]
    fn can_encode_add_reg_to_reg() {
        assert_encodes_as(
            Aarch64Instruction::AddRegToReg {
                destination: Register::X0,
                reg1: Register::X9,
                reg2: Register::X10,
            },
            vec![0x20, 0x01, 0x0A, 0x8B],
        );
    }

    #[test]
    fn can_encode_sub_reg_to_reg() {
        assert_encodes_as(
            Aarch64Instruction::SubRegToReg {
                destination: Register::X0,
                reg1: Register::X9,
                reg2: Register::X10,
            },
            vec![0x20, 0x01, 0x0A, 0xEB],
        );
    }

    #[test]
    fn can_encode_mul_reg_to_reg() {
        assert_encodes_as(
            Aarch64Instruction::MulRegToReg {
                destination: Register::X0,
                reg1: Register::X9,
                reg2: Register::X10,
            },
            vec![0x20, 0x7D, 0x0A, 0x9B],
        );
    }

    #[test]
    fn can_encode_div_reg_to_reg() {
        assert_encodes_as(
            Aarch64Instruction::DivRegToReg {
                destination: Register::X0,
                reg1: Register::X9,
                reg2: Register::X10,
            },
            vec![0x20, 0x0D, 0xCA, 0x9A],
        );
    }

    #[test]
    fn can_encode_blr() {
        assert_encodes_as(
            Aarch64Instruction::Blr {
                register: Register::X1,
            },
            vec![0x20, 0x00, 0x3F, 0xD6],
        );
    }

    #[test]
    fn can_encode_str() {
        assert_encodes_as(
            Aarch64Instruction::Str {
                source: Register::X0,
                base: Register::X0,
                offset: 0,
            },
            vec![0x00, 0x00, 0x00, 0xF9],
        );
        assert_encodes_as(
            Aarch64Instruction::Str {
                source: Register::X1,
                base: Register::X29,
                offset: 0,
            },
            vec![0xA1, 0x03, 0x00, 0xF9],
        );
        assert_encodes_as(
            Aarch64Instruction::Str {
                source: Register::X4,
                base: Register::X5,
                offset: 32,
            },
            vec![0xA4, 0x10, 0x00, 0xF9],
        );
    }

    #[test]
    fn can_encode_ldr() {
        assert_encodes_as(
            Aarch64Instruction::Ldr {
                destination: Register::X0,
                base: Register::X0,
                offset: 0,
            },
            vec![0x00, 0x00, 0x40, 0xF9],
        );
        assert_encodes_as(
            Aarch64Instruction::Ldr {
                destination: Register::X1,
                base: Register::X29,
                offset: 0,
            },
            vec![0xA1, 0x03, 0x40, 0xF9],
        );
        assert_encodes_as(
            Aarch64Instruction::Ldr {
                destination: Register::X4,
                base: Register::X5,
                offset: 32,
            },
            vec![0xA4, 0x10, 0x40, 0xF9],
        );
    }

    #[test]
    fn can_encode_stp() {
        assert_encodes_as(
            Aarch64Instruction::Stp {
                reg1: Register::X0,
                reg2: Register::X0,
                base: Register::X0,
                offset: 8,
                pre_indexing: false,
            },
            vec![0x00, 0x80, 0x00, 0xA9],
        );
        assert_encodes_as(
            Aarch64Instruction::Stp {
                reg1: Register::X0,
                reg2: Register::X0,
                base: Register::X0,
                offset: -8,
                pre_indexing: false,
            },
            vec![0x00, 0x80, 0x3F, 0xA9],
        );
        assert_encodes_as(
            Aarch64Instruction::Stp {
                reg1: Register::X2,
                reg2: Register::X0,
                base: Register::X0,
                offset: 0,
                pre_indexing: false,
            },
            vec![0x02, 0x00, 0x00, 0xA9],
        );
        assert_encodes_as(
            Aarch64Instruction::Stp {
                reg1: Register::X0,
                reg2: Register::X2,
                base: Register::X0,
                offset: 0,
                pre_indexing: false,
            },
            vec![0x00, 0x08, 0x00, 0xA9],
        );
        assert_encodes_as(
            Aarch64Instruction::Stp {
                reg1: Register::X0,
                reg2: Register::X0,
                base: Register::X2,
                offset: 0,
                pre_indexing: false,
            },
            vec![0x40, 0x00, 0x00, 0xA9],
        );
        assert_encodes_as(
            Aarch64Instruction::Stp {
                reg1: Register::X29,
                reg2: Register::X30,
                base: Register::Sp,
                offset: -16,
                pre_indexing: false,
            },
            vec![0xFD, 0x7B, 0x3F, 0xA9],
        );
    }

    #[test]
    fn can_encode_ldp() {
        assert_encodes_as(
            Aarch64Instruction::Ldp {
                reg1: Register::X29,
                reg2: Register::X30,
                base: Register::Sp,
                offset: 32,
            },
            vec![0xFD, 0x7B, 0xC2, 0xA8],
        );
    }

    #[test]
    fn can_compile_trivial_function() {
        let program = parse_program("fn main() { let a = 42; return a; }").unwrap();
        let compiled = frontend::compile(program).unwrap();
        assert_eq!(compiled.len(), 1);

        let mut gen = Aarch64Generator::default();
        let machine_code = gen
            .generate_machine_code(
                &compiled[0],
                &Box::new(CompiledFunctionCatalog::new(&compiled)),
            )
            .unwrap();
        assert_eq!(
            vec![0x48, 0x05, 0x80, 0xD2, 0xE0, 0x03, 0x08, 0xAA, 0xC0, 0x03, 0x5F, 0xD6],
            machine_code.machine_code
        );
    }

    #[test]
    fn can_compile_math() {
        let program =
            parse_program("fn the_answer() { let a = 3; return a + 1 - 2 * 3 / 4; }").unwrap();
        let compiled = frontend::compile(program).unwrap();
        assert_eq!(compiled.len(), 1);

        let mut gen = Aarch64Generator::default();
        let machine_code = gen
            .generate_machine_code(
                &compiled[0],
                &Box::new(CompiledFunctionCatalog::new(&compiled)),
            )
            .unwrap();
        assert_eq!(
            "
            |movz x8, 3
            |movz x9, 1
            |add  x10, x8, x9
            |movz x9, 2
            |movz x8, 3
            |mul  x11, x9, x8
            |movz x9, 4
            |sdiv x8, x11, x9
            |subs x11, x10, x8
            |mov  x0, x11
            |ret
            |"
            .trim_margin()
            .unwrap(),
            machine_code.asm
        );
        assert_eq!(
            vec![
                0x68, 0x00, 0x80, 0xD2, 0x29, 0x00, 0x80, 0xD2, 0x0A, 0x01, 0x09, 0x8B, 0x49, 0x00,
                0x80, 0xD2, 0x68, 0x00, 0x80, 0xD2, 0x2B, 0x7D, 0x08, 0x9B, 0x89, 0x00, 0x80, 0xD2,
                0x68, 0x0D, 0xC9, 0x9A, 0x4B, 0x01, 0x08, 0xEB, 0xE0, 0x03, 0x0B, 0xAA, 0xC0, 0x03,
                0x5F, 0xD6,
            ],
            machine_code.machine_code
        );
    }

    proptest! {
        #[test]
        fn mov_immediate_uses_one_instruction_for_16bit_values(n in 0..0xFFFF) {
            let instruction = Aarch64Instruction::MovImmToReg { register: Register::X0, value: n as f64 };
            let machine_code = instruction.make_machine_code();
            assert_eq!(4, machine_code.len());
        }

        #[test]
        fn mov_immediate_uses_two_instructions_for_32bit_values(n in 0x10000..0xFFFFFFFFu32) {
            let instruction = Aarch64Instruction::MovImmToReg { register: Register::X0, value: n as f64 };
            let machine_code = instruction.make_machine_code();
            assert_eq!(8, machine_code.len());
        }

        #[test]
        fn mov_immediate_uses_three_instructions_for_48bit_values(n in 0x100000000..0xFFFFFFFFFFFFu64) {
            let instruction = Aarch64Instruction::MovImmToReg { register: Register::X0, value: n as f64 };
            let machine_code = instruction.make_machine_code();
            assert_eq!(12, machine_code.len());
        }

        #[test]
        fn mov_immediate_uses_four_instructions_for_64bit_values(n in 0x1000000000000..0xFFFFFFFFFFFFFFFFu64) {
            let instruction = Aarch64Instruction::MovImmToReg { register: Register::X0, value: n as f64 };
            let machine_code = instruction.make_machine_code();
            assert_eq!(16, machine_code.len());
        }
    }
}
