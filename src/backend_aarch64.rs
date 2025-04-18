use std::{
    cmp::max,
    fmt::{Display, Write},
};

use crate::{
    backend::{BackendError, CompiledFunctionCatalog, GeneratedMachineCode, MachineCodeGenerator},
    backend_register_allocator::{self, AllocatedLocation},
    ir::{ArgumentIndex, BinOpOperator::*, CompiledFunction, IrInstruction},
    jit::jit_call_trampoline,
};
use Aarch64Instruction::*;
use Register::*;

#[derive(Debug, Clone, Copy, PartialEq)]
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
            X0 => 0,
            X1 => 1,
            X2 => 2,
            X3 => 3,
            X4 => 4,
            X5 => 5,
            X6 => 6,
            X7 => 7,
            X8 => 8,
            X9 => 9,
            X10 => 10,
            X11 => 11,
            X12 => 12,
            X13 => 13,
            X14 => 14,
            X15 => 15,
            X16 => 16,
            X17 => 17,
            X18 => 18,
            X19 => 19,
            X20 => 20,
            X21 => 21,
            X22 => 22,
            X23 => 23,
            X24 => 24,
            X25 => 25,
            X26 => 26,
            X27 => 27,
            X28 => 28,
            X29 => 29,
            X30 => 30,
            Sp => 31,
        }
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            X0 => write!(f, "x0"),
            X1 => write!(f, "x1"),
            X2 => write!(f, "x2"),
            X3 => write!(f, "x3"),
            X4 => write!(f, "x4"),
            X5 => write!(f, "x5"),
            X6 => write!(f, "x6"),
            X7 => write!(f, "x7"),
            X8 => write!(f, "x8"),
            X9 => write!(f, "x9"),
            X10 => write!(f, "x10"),
            X11 => write!(f, "x11"),
            X12 => write!(f, "x12"),
            X13 => write!(f, "x13"),
            X14 => write!(f, "x14"),
            X15 => write!(f, "x15"),
            X16 => write!(f, "x16"),
            X17 => write!(f, "x17"),
            X18 => write!(f, "x18"),
            X19 => write!(f, "x19"),
            X20 => write!(f, "x20"),
            X21 => write!(f, "x21"),
            X22 => write!(f, "x22"),
            X23 => write!(f, "x23"),
            X24 => write!(f, "x24"),
            X25 => write!(f, "x25"),
            X26 => write!(f, "x26"),
            X27 => write!(f, "x27"),
            X28 => write!(f, "x28"),
            X29 => write!(f, "x29"),
            X30 => write!(f, "x30"),
            Sp => write!(f, "sp"),
        }
    }
}

enum Aarch64Instruction {
    Nop,
    Ret,
    MovImmToReg {
        register: Register,
        value: i64,
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
    Neg {
        source: Register,
        destination: Register,
    },
}

impl Display for Aarch64Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Nop => write!(f, "nop"),
            Ret => write!(f, "ret"),
            MovImmToReg { register, value } => {
                write!(f, "movz {}, {}", register, value)
            }
            MovRegToReg {
                source,
                destination,
            } => {
                write!(f, "mov  {}, {}", destination, source)
            }
            MovSpToReg { destination } => {
                write!(f, "mov  {}, sp", destination)
            }
            AddRegToReg {
                destination,
                reg1,
                reg2,
            } => write!(f, "add  {}, {}, {}", destination, reg1, reg2),
            SubRegToReg {
                destination,
                reg1,
                reg2,
            } => write!(f, "subs {}, {}, {}", destination, reg1, reg2),
            MulRegToReg {
                destination,
                reg1,
                reg2,
            } => write!(f, "mul  {}, {}, {}", destination, reg1, reg2),
            DivRegToReg {
                destination,
                reg1,
                reg2,
            } => write!(f, "sdiv {}, {}, {}", destination, reg1, reg2),
            Blr { register } => write!(f, "blr {}", register),
            Str {
                source,
                base,
                offset,
            } => write!(f, "str  {}, [{}, #{}]", source, base, offset),
            Ldr {
                destination,
                base,
                offset,
            } => write!(f, "ldr  {}, [{}, #{}]", destination, base, offset),
            Stp {
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
            Ldp {
                reg1,
                reg2,
                base,
                offset,
            } => write!(f, "ldp  {}, {}, [{}], #{}", reg1, reg2, base, offset),
            Neg {
                source,
                destination,
            } => write!(f, "neg  {}, {}", destination, source),
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
    const NEG: u32 = 0xCB0003E0;

    fn make_machine_code(&self) -> Vec<u8> {
        match self {
            Nop => vec![0xD5, 0x03, 0x20, 0x1F],
            Ret => vec![0xC0, 0x03, 0x5F, 0xD6],

            MovImmToReg { register, value } => {
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

            MovRegToReg {
                source,
                destination,
            } => {
                let mut i: u32 = Self::MOV;
                i |= source.index() << 16;
                i |= destination.index();
                i.to_le_bytes().to_vec()
            }

            MovSpToReg { destination } => {
                let mut i: u32 = Self::MOV_SP_TO_REG;
                i |= destination.index();
                i.to_le_bytes().to_vec()
            }

            AddRegToReg {
                destination,
                reg1,
                reg2,
            } => Self::encode_three_reg_op(Self::ADD, destination, reg1, reg2),

            SubRegToReg {
                destination,
                reg1,
                reg2,
            } => Self::encode_three_reg_op(Self::SUBS, destination, reg1, reg2),

            MulRegToReg {
                destination,
                reg1,
                reg2,
            } => Self::encode_three_reg_op(Self::MUL, destination, reg1, reg2),

            DivRegToReg {
                destination,
                reg1,
                reg2,
            } => Self::encode_three_reg_op(Self::SDIV, destination, reg1, reg2),

            Blr { register } => {
                let mut i = Self::BLR;
                i |= register.index() << 5;
                i.to_le_bytes().to_vec()
            }

            Str {
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

            Ldr {
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

            Stp {
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
                let offset: u32 = unsafe { std::mem::transmute((offset >> 3) & 0x7F) };
                i |= offset << 15;
                i.to_le_bytes().to_vec()
            }

            Ldp {
                reg1,
                reg2,
                base,
                offset,
            } => {
                let mut i = Self::LDP;
                i |= reg1.index();
                i |= reg2.index() << 10;
                i |= base.index() << 5;
                let offset: u32 = unsafe { std::mem::transmute((offset >> 3) & 0x7F) };
                i |= offset << 15;
                i.to_le_bytes().to_vec()
            }

            Neg {
                source,
                destination,
            } => {
                let mut i: u32 = Self::NEG;
                i |= source.index() << 16;
                i |= destination.index();
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
    max_stack_offset: u32,
    used_registers: Vec<Register>,
    used_args_registers: Vec<Register>,
}

impl MachineCodeGenerator for Aarch64Generator {
    fn generate_machine_code(
        &mut self,
        function: &CompiledFunction,
        function_catalog: &CompiledFunctionCatalog,
    ) -> Result<GeneratedMachineCode, BackendError> {
        self.allocate_registers(function);
        self.compute_used_args_registers(function)?;

        let mut instructions = Vec::new();
        let mut index_of_ldp_to_fix = Vec::new();
        self.stack_offset += 16;
        self.max_stack_offset = self.stack_offset;

        // This will be overwritten at the end, once we have completed computation
        // of the necessary stack depth
        instructions.push(Nop);
        instructions.push(MovSpToReg { destination: X29 });

        for instruction in function.body.iter() {
            match instruction {
                IrInstruction::Mvi { dest, val } => {
                    let AllocatedLocation::Register { register } = self.locations[dest.0] else {
                        return Err(BackendError::NotImplemented(
                            "move immediate to stack".to_string(),
                        ));
                    };

                    instructions.push(MovImmToReg {
                        register,
                        value: *val,
                    })
                }

                IrInstruction::MvArg { dest, arg } => {
                    let location = Self::get_argument_location(*arg)?;
                    let AllocatedLocation::Register { register: source } = location else {
                        return Err(BackendError::NotImplemented(
                            "move argument from stack".to_string(),
                        ));
                    };

                    let AllocatedLocation::Register {
                        register: destination,
                    } = self.locations[dest.0]
                    else {
                        return Err(BackendError::NotImplemented(
                            "move argument to stack".to_string(),
                        ));
                    };

                    instructions.push(MovRegToReg {
                        source,
                        destination,
                    });
                }

                IrInstruction::Ret { reg } => {
                    let AllocatedLocation::Register { register: source } = self.locations[reg.0]
                    else {
                        return Err(BackendError::NotImplemented(
                            "return value from stack".to_string(),
                        ));
                    };

                    instructions.push(MovRegToReg {
                        source,
                        destination: X0,
                    });

                    // We will replace this with the correct LDP at the end,
                    // once the final stack depth has been computed
                    index_of_ldp_to_fix.push(instructions.len());
                    instructions.push(Nop);

                    instructions.push(Ret);
                }

                IrInstruction::Neg { dest, op } => {
                    let AllocatedLocation::Register { register: source } = self.locations[op.0]
                    else {
                        return Err(BackendError::NotImplemented(
                            "negate stack value".to_string(),
                        ));
                    };

                    let AllocatedLocation::Register {
                        register: destination,
                    } = self.locations[dest.0]
                    else {
                        return Err(BackendError::NotImplemented(
                            "store negation to stack value".to_string(),
                        ));
                    };

                    instructions.push(Neg {
                        destination,
                        source,
                    });
                }

                IrInstruction::BinOp {
                    operator,
                    dest,
                    op1,
                    op2,
                } => {
                    let AllocatedLocation::Register { register: reg1 } = self.locations[op1.0]
                    else {
                        return Err(BackendError::NotImplemented(
                            "binop when one operand is in stack".to_string(),
                        ));
                    };
                    let AllocatedLocation::Register { register: reg2 } = self.locations[op2.0]
                    else {
                        return Err(BackendError::NotImplemented(
                            "binop when one operand is in stack".to_string(),
                        ));
                    };
                    let AllocatedLocation::Register {
                        register: destination,
                    } = self.locations[dest.0]
                    else {
                        return Err(BackendError::NotImplemented(
                            "binop when destination is in stack".to_string(),
                        ));
                    };

                    instructions.push(match operator {
                        Add => AddRegToReg {
                            destination,
                            reg1,
                            reg2,
                        },
                        Sub => SubRegToReg {
                            destination,
                            reg1,
                            reg2,
                        },
                        Mul => MulRegToReg {
                            destination,
                            reg1,
                            reg2,
                        },
                        Div => DivRegToReg {
                            destination,
                            reg1,
                            reg2,
                        },
                    });
                }

                IrInstruction::Call {
                    dest,
                    name: _,
                    function_id: called_function_id,
                    args: call_args,
                } => {
                    let fn_catalog_addr: usize =
                        function_catalog as *const CompiledFunctionCatalog as usize;
                    let jit_call_trampoline_address: usize = jit_call_trampoline as usize;

                    self.push(&mut instructions, X0);

                    // We will put the jump address in X19
                    self.push(&mut instructions, X19);

                    // Store all registers being used. We should skip the destination one
                    // for this instruction, since we will overwrite it, but whatever.
                    // We generate horrible code anyway... what's one more push/pop pair? :-D
                    let used_registers = self.used_registers.clone();
                    for used_register in used_registers.iter().cloned() {
                        self.push(&mut instructions, used_register);
                    }
                    let used_args_registers = self.used_args_registers.clone();
                    for used_arg_register in used_args_registers.iter().cloned() {
                        if used_arg_register != X0 {
                            self.push(&mut instructions, used_arg_register);
                        }
                    }

                    // jit_call_trampoline(function_catalog_ptr, called_function_index, args)
                    instructions.push(MovImmToReg {
                        register: X0,
                        value: fn_catalog_addr as i64,
                    });
                    instructions.push(MovImmToReg {
                        register: X1,
                        value: called_function_id.0 as i64,
                    });

                    // Fill arguments
                    for (call_arg, actual_arg) in call_args.iter().enumerate() {
                        let shifted_call_arg = call_arg + 2; // X0 and X1 are already used
                        let AllocatedLocation::Register {
                            register: actual_arg_register,
                        } = self.locations[actual_arg.0]
                        else {
                            return Err(BackendError::NotImplemented(
                                "passing arguments to function from stack".to_string(),
                            ));
                        };

                        let arg_location = Self::get_argument_location((shifted_call_arg).into())?;
                        let AllocatedLocation::Register {
                            register: call_convention_arg_register,
                        } = arg_location
                        else {
                            return Err(BackendError::NotImplemented(
                                "functions with more than 8 arguments".to_string(),
                            ));
                        };

                        instructions.push(MovRegToReg {
                            source: actual_arg_register,
                            destination: call_convention_arg_register,
                        });
                    }
                    instructions.push(MovImmToReg {
                        register: X19,
                        value: jit_call_trampoline_address as i64,
                    });

                    // We can finally do the actual call!
                    instructions.push(Blr { register: X19 });

                    // Restore registers
                    for used_arg_register in used_args_registers.iter().cloned() {
                        if used_arg_register != X0 {
                            self.pop(&mut instructions, used_arg_register);
                        }
                    }
                    for used_register in used_registers.iter().rev().cloned() {
                        self.pop(&mut instructions, used_register);
                    }
                    self.pop(&mut instructions, X19);

                    // Copy result (x0) to the opportune register
                    let AllocatedLocation::Register {
                        register: destination,
                    } = self.locations[dest.0]
                    else {
                        return Err(BackendError::NotImplemented(
                            "move register to stack".to_string(),
                        ));
                    };

                    instructions.push(MovRegToReg {
                        source: X0,
                        destination,
                    });

                    self.pop(&mut instructions, X0);
                }
            }
        }

        // Replace the prologue and epilogue, now that we know the maximum stack depth
        let stack_depth_to_reserve = (self.max_stack_offset + 15) & 0xFFFFFFF0; // Must be 16-byte aligned
        instructions[0] = Stp {
            reg1: X29,
            reg2: X30,
            base: Sp,
            offset: -(stack_depth_to_reserve as i32),
            pre_indexing: true,
        };
        for ldp_to_fix_index in index_of_ldp_to_fix {
            instructions[ldp_to_fix_index] = Ldp {
                reg1: X29,
                reg2: X30,
                base: Sp,
                offset: stack_depth_to_reserve as i32,
            };
        }

        // Done!
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
                // Caller-seved registers only
                // TODO: add X19-X28 (callee-saved registers) and save them before modifying
                X9, X10, X11, X12, X13, X14, X15,
            ],
        );
        self.locations = allocations;

        for location in self.locations.iter() {
            if let AllocatedLocation::Register { register } = location {
                // This looks quadratic, but actually we only have 7 registers.
                // Therefore this is actually 7 * N i.e. linear. Probably faster
                // than a hash set.
                // And, once again, this is a toy, not an efficient compiler!
                if !self.used_registers.contains(register) {
                    self.used_registers.push(*register);
                }
            }
        }
    }

    fn compute_used_args_registers(
        &mut self,
        function: &CompiledFunction,
    ) -> Result<(), BackendError> {
        for arg in 0..function.num_args {
            let location = Self::get_argument_location(arg.into())?;
            match location {
                AllocatedLocation::Register { register } => {
                    self.used_args_registers.push(register);
                }
                AllocatedLocation::Stack { offset: _ } => {
                    return Err(BackendError::NotImplemented(
                        "functions with more than 8 arguments".to_string(),
                    ))
                }
            }
        }
        Ok(())
    }

    fn push(&mut self, instructions: &mut Vec<Aarch64Instruction>, register: Register) {
        self.stack_offset += 8;
        self.max_stack_offset = max(self.max_stack_offset, self.stack_offset);
        instructions.push(Str {
            source: register,
            base: X29,
            offset: self.stack_offset,
        });
    }

    fn pop(&mut self, instructions: &mut Vec<Aarch64Instruction>, register: Register) {
        instructions.push(Ldr {
            destination: register,
            base: X29,
            offset: self.stack_offset,
        });
        self.stack_offset -= 8;
    }

    fn get_argument_location(
        arg: ArgumentIndex,
    ) -> Result<AllocatedLocation<Register>, BackendError> {
        let arg: usize = arg.into();
        // Should probably use some macro...
        match arg {
            0 => Ok(AllocatedLocation::Register { register: X0 }),
            1 => Ok(AllocatedLocation::Register { register: X1 }),
            2 => Ok(AllocatedLocation::Register { register: X2 }),
            3 => Ok(AllocatedLocation::Register { register: X3 }),
            4 => Ok(AllocatedLocation::Register { register: X4 }),
            5 => Ok(AllocatedLocation::Register { register: X5 }),
            6 => Ok(AllocatedLocation::Register { register: X6 }),
            7 => Ok(AllocatedLocation::Register { register: X7 }),
            _ => Err(BackendError::NotImplemented(
                "support for more than 8 arguments".to_string(),
            )),
        }
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
            MovImmToReg {
                register: X1,
                value: 123,
            },
            vec![0x61, 0x0F, 0x80, 0xD2],
        );
    }

    #[test]
    fn can_encode_move_immediate_32_bit() {
        assert_encodes_as(
            MovImmToReg {
                register: X1,
                value: 1234567,
            },
            vec![0xE1, 0xD0, 0x9A, 0xD2, 0x41, 0x02, 0xA0, 0xF2],
        );
    }

    #[test]
    fn can_encode_move_immediate_48_bit() {
        assert_encodes_as(
            MovImmToReg {
                register: X1,
                value: 12345678901,
            },
            vec![
                0xA1, 0x86, 0x83, 0xD2, 0x81, 0xFB, 0xBB, 0xF2, 0x41, 0x00, 0xC0, 0xF2,
            ],
        );
    }

    #[test]
    fn can_encode_move_immediate_64_bit() {
        assert_encodes_as(
            MovImmToReg {
                register: X1,
                value: 1234567890123456,
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
            MovRegToReg {
                source: X8,
                destination: X9,
            },
            vec![0xE9, 0x03, 0x08, 0xAA],
        );
    }

    #[test]
    fn can_encode_mov_sp_to_reg() {
        assert_encodes_as(
            MovSpToReg { destination: X29 },
            vec![0xFD, 0x03, 0x00, 0x91],
        );
    }

    #[test]
    fn can_encode_add_reg_to_reg() {
        assert_encodes_as(
            AddRegToReg {
                destination: X0,
                reg1: X9,
                reg2: X10,
            },
            vec![0x20, 0x01, 0x0A, 0x8B],
        );
    }

    #[test]
    fn can_encode_sub_reg_to_reg() {
        assert_encodes_as(
            SubRegToReg {
                destination: X0,
                reg1: X9,
                reg2: X10,
            },
            vec![0x20, 0x01, 0x0A, 0xEB],
        );
    }

    #[test]
    fn can_encode_mul_reg_to_reg() {
        assert_encodes_as(
            MulRegToReg {
                destination: X0,
                reg1: X9,
                reg2: X10,
            },
            vec![0x20, 0x7D, 0x0A, 0x9B],
        );
    }

    #[test]
    fn can_encode_div_reg_to_reg() {
        assert_encodes_as(
            DivRegToReg {
                destination: X0,
                reg1: X9,
                reg2: X10,
            },
            vec![0x20, 0x0D, 0xCA, 0x9A],
        );
    }

    #[test]
    fn can_encode_blr() {
        assert_encodes_as(Blr { register: X1 }, vec![0x20, 0x00, 0x3F, 0xD6]);
    }

    #[test]
    fn can_encode_str() {
        assert_encodes_as(
            Str {
                source: X0,
                base: X0,
                offset: 0,
            },
            vec![0x00, 0x00, 0x00, 0xF9],
        );
        assert_encodes_as(
            Str {
                source: X1,
                base: X29,
                offset: 0,
            },
            vec![0xA1, 0x03, 0x00, 0xF9],
        );
        assert_encodes_as(
            Str {
                source: X4,
                base: X5,
                offset: 32,
            },
            vec![0xA4, 0x10, 0x00, 0xF9],
        );
    }

    #[test]
    fn can_encode_ldr() {
        assert_encodes_as(
            Ldr {
                destination: X0,
                base: X0,
                offset: 0,
            },
            vec![0x00, 0x00, 0x40, 0xF9],
        );
        assert_encodes_as(
            Ldr {
                destination: X1,
                base: X29,
                offset: 0,
            },
            vec![0xA1, 0x03, 0x40, 0xF9],
        );
        assert_encodes_as(
            Ldr {
                destination: X4,
                base: X5,
                offset: 32,
            },
            vec![0xA4, 0x10, 0x40, 0xF9],
        );
    }

    #[test]
    fn can_encode_stp() {
        assert_encodes_as(
            Stp {
                reg1: X0,
                reg2: X0,
                base: X0,
                offset: 8,
                pre_indexing: false,
            },
            vec![0x00, 0x80, 0x00, 0xA9],
        );
        assert_encodes_as(
            Stp {
                reg1: X0,
                reg2: X0,
                base: X0,
                offset: -8,
                pre_indexing: false,
            },
            vec![0x00, 0x80, 0x3F, 0xA9],
        );
        assert_encodes_as(
            Stp {
                reg1: X2,
                reg2: X0,
                base: X0,
                offset: 0,
                pre_indexing: false,
            },
            vec![0x02, 0x00, 0x00, 0xA9],
        );
        assert_encodes_as(
            Stp {
                reg1: X0,
                reg2: X2,
                base: X0,
                offset: 0,
                pre_indexing: false,
            },
            vec![0x00, 0x08, 0x00, 0xA9],
        );
        assert_encodes_as(
            Stp {
                reg1: X0,
                reg2: X0,
                base: X2,
                offset: 0,
                pre_indexing: false,
            },
            vec![0x40, 0x00, 0x00, 0xA9],
        );
        assert_encodes_as(
            Stp {
                reg1: X29,
                reg2: X30,
                base: Sp,
                offset: -16,
                pre_indexing: false,
            },
            vec![0xFD, 0x7B, 0x3F, 0xA9],
        );
    }

    #[test]
    fn can_encode_ldp() {
        assert_encodes_as(
            Ldp {
                reg1: X29,
                reg2: X30,
                base: Sp,
                offset: 32,
            },
            vec![0xFD, 0x7B, 0xC2, 0xA8],
        );
    }

    #[test]
    fn can_encode_neg() {
        assert_encodes_as(
            Neg {
                source: X5,
                destination: X1,
            },
            vec![0xE1, 0x03, 0x05, 0xCB],
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
            "
            |stp  x29, x30, [sp, #-16]!
            |mov  x29, sp
            |movz x9, 42
            |mov  x0, x9
            |ldp  x29, x30, [sp], #16
            |ret
            |"
            .trim_margin()
            .unwrap(),
            machine_code.asm
        );
        assert_eq!(
            vec![
                0xFD, 0x7B, 0xBF, 0xA9, 0xFD, 0x03, 0x00, 0x91, 0x49, 0x05, 0x80, 0xD2, 0xE0, 0x03,
                0x09, 0xAA, 0xFD, 0x7B, 0xC1, 0xA8, 0xC0, 0x03, 0x5F, 0xD6
            ],
            machine_code.machine_code
        );
    }

    #[test]
    fn can_compile_math() {
        let program =
            parse_program("fn the_answer() { let a = 3; return a + 1 - 2 * 3 / -4; }").unwrap();
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
            |stp  x29, x30, [sp, #-16]!
            |mov  x29, sp
            |movz x9, 3
            |movz x10, 1
            |add  x11, x9, x10
            |movz x10, 2
            |movz x9, 3
            |mul  x12, x10, x9
            |movz x10, 4
            |neg  x9, x10
            |sdiv x10, x12, x9
            |subs x12, x11, x10
            |mov  x0, x12
            |ldp  x29, x30, [sp], #16
            |ret
            |"
            .trim_margin()
            .unwrap(),
            machine_code.asm
        );
    }

    #[test]
    fn can_compile_function_calls() {
        let program = parse_program(
            "
            fn f() { return 1 + g(); }
            fn g() { return 42; }
            ",
        )
        .unwrap();
        let compiled = frontend::compile(program).unwrap();
        assert_eq!(compiled.len(), 2);

        let function_catalog = Box::new(CompiledFunctionCatalog::new(&compiled));
        let fn_catalog_addr: usize =
            function_catalog.as_ref() as *const CompiledFunctionCatalog as usize;
        let jit_call_trampoline_address: usize = jit_call_trampoline as usize;

        let mut gen = Aarch64Generator::default();
        let machine_code = gen
            .generate_machine_code(
                &compiled[0], // f
                &function_catalog,
            )
            .unwrap();
        assert_eq!(
            format!(
                "
            |stp  x29, x30, [sp, #-64]!
            |mov  x29, sp
            |movz x9, 1
            |str  x0, [x29, #24]
            |str  x19, [x29, #32]
            |str  x9, [x29, #40]
            |str  x10, [x29, #48]
            |str  x11, [x29, #56]
            |movz x0, {}
            |movz x1, 1
            |movz x19, {}
            |blr x19
            |ldr  x11, [x29, #56]
            |ldr  x10, [x29, #48]
            |ldr  x9, [x29, #40]
            |ldr  x19, [x29, #32]
            |mov  x10, x0
            |ldr  x0, [x29, #24]
            |add  x11, x9, x10
            |mov  x0, x11
            |ldp  x29, x30, [sp], #64
            |ret
            |",
                fn_catalog_addr, jit_call_trampoline_address
            )
            .trim_margin()
            .unwrap(),
            machine_code.asm
        );
    }

    proptest! {
        #[test]
        fn mov_immediate_uses_one_instruction_for_16bit_values(n in 0..0xFFFF) {
            let instruction = MovImmToReg { register: X0, value: n as i64 };
            let machine_code = instruction.make_machine_code();
            assert_eq!(4, machine_code.len());
        }

        #[test]
        fn mov_immediate_uses_two_instructions_for_32bit_values(n in 0x10000..0xFFFFFFFFu32) {
            let instruction = MovImmToReg { register: X0, value: n as i64 };
            let machine_code = instruction.make_machine_code();
            assert_eq!(8, machine_code.len());
        }

        #[test]
        fn mov_immediate_uses_three_instructions_for_48bit_values(n in 0x100000000..0xFFFFFFFFFFFFu64) {
            let instruction = MovImmToReg { register: X0, value: n as i64 };
            let machine_code = instruction.make_machine_code();
            assert_eq!(12, machine_code.len());
        }

        #[test]
        fn mov_immediate_uses_four_instructions_for_64bit_values(n in 0x1000000000000..0xFFFFFFFFFFFFFFFFu64) {
            let instruction = MovImmToReg { register: X0, value: n as i64 };
            let machine_code = instruction.make_machine_code();
            assert_eq!(16, machine_code.len());
        }
    }
}
