use std::fmt::{Display, Write};

use crate::{
    backend::{GeneratedMachineCode, MachineCodeGenerator},
    ir::{CompiledFunction, Instruction, RegisterIndex},
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
}

impl Register {
    fn index(&self) -> u8 {
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
        }
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Register::X0 => write!(f, "x0"),
            Register::X1 => write!(f, "x2"),
            Register::X2 => write!(f, "x3"),
            Register::X3 => write!(f, "x4"),
            Register::X4 => write!(f, "x5"),
            Register::X5 => write!(f, "x6"),
            Register::X6 => write!(f, "x7"),
            Register::X7 => write!(f, "x1"),
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
        }
    }
}

enum Aarch64Instruction {
    Ret,
    MovImmToReg {
        register: Register,
        value: f64,
    },
    MovRegToReg {
        source: Register,
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
}

impl Display for Aarch64Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
            Aarch64Instruction::AddRegToReg {
                destination,
                reg1,
                reg2,
            } => write!(f, "add  {}, {}, {}", destination, reg1, reg2),

            Aarch64Instruction::SubRegToReg {
                destination,
                reg1,
                reg2,
            } => write!(f, "subs  {}, {}, {}", destination, reg1, reg2),

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
        }
    }
}

impl Aarch64Instruction {
    const MOVZ: u32 = 0xD2800000;
    const MOVK_SHIFT_16: u32 = 0xF2A00000;
    const MOVK_SHIFT_32: u32 = 0xF2C00000;
    const MOVK_SHIFT_48: u32 = 0xF2E00000;

    const MOV: u32 = 0xAA0003E0;
    const ADD: u32 = 0x8B000000;
    const SUBS: u32 = 0xEB000000;
    const MUL: u32 = 0x9B007C00;
    const SDIV: u32 = 0x9AC00C00;

    fn make_machine_code(&self) -> Vec<u8> {
        match self {
            Aarch64Instruction::Ret => vec![0xc0, 0x03, 0x5f, 0xd6],

            Aarch64Instruction::MovImmToReg { register, value } => {
                // Note: there are a lot more efficient encoding: for example, we always
                // use 64 bit registers here, and we could use the bitmask immediate
                // trick described here:
                // https://kddnewton.com/2022/08/11/aarch64-bitmask-immediates.html
                // But, since this is a toy, I don't really care about efficiency. :-)
                let int_value = *value as u64;

                if int_value < 0xFFFF {
                    let mut i0 = Self::MOVZ;
                    i0 |= ((int_value & 0xFFFF) as u32) << 5;
                    i0 |= register.index() as u32;

                    i0.to_le_bytes().to_vec()
                } else if int_value < 0xFFFFFFFF {
                    let mut i0 = Self::MOVZ;
                    i0 |= ((int_value & 0xFFFF) as u32) << 5;
                    i0 |= register.index() as u32;

                    let mut i1 = Self::MOVK_SHIFT_16;
                    i1 |= (((int_value >> 16) & 0xFFFF) as u32) << 5;
                    i1 |= register.index() as u32;

                    let mut v: Vec<u8> = Vec::with_capacity(8);
                    v.extend(i0.to_le_bytes());
                    v.extend(i1.to_le_bytes());
                    v
                } else if int_value < 0xFFFFFFFFFFFF {
                    let mut i0: u32 = Self::MOVZ;
                    i0 |= ((int_value & 0xFFFF) as u32) << 5;
                    i0 |= register.index() as u32;

                    let mut i1 = Self::MOVK_SHIFT_16;
                    i1 |= (((int_value >> 16) & 0xFFFF) as u32) << 5;
                    i1 |= register.index() as u32;

                    let mut i2 = Self::MOVK_SHIFT_32;
                    i2 |= (((int_value >> 32) & 0xFFFF) as u32) << 5;
                    i2 |= register.index() as u32;

                    let mut v: Vec<u8> = Vec::with_capacity(12);
                    v.extend(i0.to_le_bytes());
                    v.extend(i1.to_le_bytes());
                    v.extend(i2.to_le_bytes());
                    v
                } else {
                    let mut i0: u32 = Self::MOVZ;
                    i0 |= ((int_value & 0xFFFF) as u32) << 5;
                    i0 |= register.index() as u32;

                    let mut i1 = Self::MOVK_SHIFT_16;
                    i1 |= (((int_value >> 16) & 0xFFFF) as u32) << 5;
                    i1 |= register.index() as u32;

                    let mut i2 = Self::MOVK_SHIFT_32;
                    i2 |= (((int_value >> 32) & 0xFFFF) as u32) << 5;
                    i2 |= register.index() as u32;

                    let mut i3 = Self::MOVK_SHIFT_48;
                    i3 |= (((int_value >> 48) & 0xFFFF) as u32) << 5;
                    i3 |= register.index() as u32;

                    let mut v: Vec<u8> = Vec::with_capacity(16);
                    v.extend(i0.to_le_bytes());
                    v.extend(i1.to_le_bytes());
                    v.extend(i2.to_le_bytes());
                    v.extend(i3.to_le_bytes());
                    v
                }
            }

            Aarch64Instruction::MovRegToReg {
                source,
                destination,
            } => {
                let mut i: u32 = Self::MOV;
                i |= (source.index() as u32) << 16;
                i |= destination.index() as u32;
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
        }
    }

    fn encode_three_reg_op(
        base: u32,
        destination: &Register,
        reg1: &Register,
        reg2: &Register,
    ) -> Vec<u8> {
        let mut i: u32 = base;
        i |= (reg1.index() as u32) << 5;
        i |= (reg2.index() as u32) << 16;
        i |= destination.index() as u32;
        i.to_le_bytes().to_vec()
    }
}

#[derive(Debug, Clone)]
enum Location {
    Register { register: Register },
    Stack { offset: usize },
}

#[derive(Default)]
pub struct Aarch64Generator {
    locations: Vec<Location>,
}

impl MachineCodeGenerator for Aarch64Generator {
    fn generate_machine_code(&mut self, function: &CompiledFunction) -> GeneratedMachineCode {
        self.allocate_registers(function);

        let mut instructions = Vec::new();
        for instruction in function.body.iter() {
            match instruction {
                Instruction::Mvi { dest, val } => {
                    let dest: usize = (*dest).into();
                    match self.locations[dest] {
                        Location::Register { register } => {
                            instructions.push(Aarch64Instruction::MovImmToReg {
                                register,
                                value: *val,
                            })
                        }
                        Location::Stack { offset: _ } => todo!(),
                    }
                }
                Instruction::Ret { reg } => {
                    let dest: usize = (*reg).into();
                    match self.locations[dest] {
                        Location::Register { register } => {
                            instructions.push(Aarch64Instruction::MovRegToReg {
                                source: register,
                                destination: Register::X0,
                            });
                        }
                        Location::Stack { offset: _ } => todo!(),
                    }
                    instructions.push(Aarch64Instruction::Ret);
                }
                Instruction::Add { dest, op1, op2 } => {
                    self.do_binop(&mut instructions, *dest, *op1, *op2, |reg1, reg2| {
                        Aarch64Instruction::AddRegToReg {
                            destination: Register::X0,
                            reg1,
                            reg2,
                        }
                    });
                }

                Instruction::Sub { dest, op1, op2 } => {
                    self.do_binop(&mut instructions, *dest, *op1, *op2, |reg1, reg2| {
                        Aarch64Instruction::SubRegToReg {
                            destination: Register::X0,
                            reg1,
                            reg2,
                        }
                    });
                }

                Instruction::Mul { dest, op1, op2 } => {
                    self.do_binop(&mut instructions, *dest, *op1, *op2, |reg1, reg2| {
                        Aarch64Instruction::MulRegToReg {
                            destination: Register::X0,
                            reg1,
                            reg2,
                        }
                    });
                }

                Instruction::Div { dest, op1, op2 } => {
                    self.do_binop(&mut instructions, *dest, *op1, *op2, |reg1, reg2| {
                        Aarch64Instruction::DivRegToReg {
                            destination: Register::X0,
                            reg1,
                            reg2,
                        }
                    });
                }

                _ => todo!(),
            }
        }

        let mut asm = String::new();
        let mut machine_code: Vec<u8> = Vec::new();

        for instruction in instructions {
            let _ = writeln!(&mut asm, "{}", instruction);
            machine_code.extend(instruction.make_machine_code());
        }

        GeneratedMachineCode { asm, machine_code }
    }
}

impl Aarch64Generator {
    // Extremely stupid algorithm - we never reuse registers!
    fn allocate_registers(&mut self, function: &CompiledFunction) {
        self.locations.reserve(function.max_used_registers.into());

        for i in 0usize..function.max_used_registers.into() {
            let location: Location = match i {
                0 => Location::Register {
                    register: Register::X8,
                },
                1 => Location::Register {
                    register: Register::X10,
                },
                2 => Location::Register {
                    register: Register::X11,
                },
                3 => Location::Register {
                    register: Register::X12,
                },
                4 => Location::Register {
                    register: Register::X13,
                },
                5 => Location::Register {
                    register: Register::X14,
                },
                6 => Location::Register {
                    register: Register::X15,
                },
                _ => Location::Stack {
                    offset: i * std::mem::size_of::<u64>(),
                },
            };
            self.locations.push(location);
        }
    }

    fn do_binop(
        &self,
        instructions: &mut Vec<Aarch64Instruction>,
        dest: RegisterIndex,
        op1: RegisterIndex,
        op2: RegisterIndex,
        callback: impl Fn(Register, Register) -> Aarch64Instruction,
    ) {
        let op1: usize = op1.into();
        let op2: usize = op2.into();

        match self.locations[op1] {
            Location::Register { register: reg1 } => match self.locations[op2] {
                Location::Register { register: reg2 } => {
                    instructions.push(callback(reg1, reg2));
                }
                Location::Stack { offset: _ } => todo!(),
            },

            Location::Stack { offset: _ } => todo!(),
        }

        let dest: usize = dest.into();
        match self.locations[dest] {
            Location::Register { register } => {
                instructions.push(Aarch64Instruction::MovRegToReg {
                    source: Register::X0,
                    destination: register,
                });
            }
            Location::Stack { offset: _ } => todo!(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{frontend, parser::*};

    #[test]
    fn can_encode_move_immediate_16_bit() {
        let instruction = Aarch64Instruction::MovImmToReg {
            register: Register::X1,
            value: 123.,
        };

        let machine_code = instruction.make_machine_code();
        assert_eq!(machine_code, vec![0x61, 0x0F, 0x80, 0xD2]);
    }

    #[test]
    fn can_encode_move_immediate_32_bit() {
        let instruction = Aarch64Instruction::MovImmToReg {
            register: Register::X1,
            value: 1234567.,
        };

        let machine_code = instruction.make_machine_code();
        assert_eq!(
            machine_code,
            vec![0xE1, 0xD0, 0x9A, 0xD2, 0x41, 0x02, 0xA0, 0xF2]
        );
    }

    #[test]
    fn can_encode_move_immediate_48_bit() {
        let instruction = Aarch64Instruction::MovImmToReg {
            register: Register::X1,
            value: 12345678901.,
        };

        let machine_code = instruction.make_machine_code();
        assert_eq!(
            machine_code,
            vec![0xA1, 0x86, 0x83, 0xD2, 0x81, 0xFB, 0xBB, 0xF2, 0x41, 0x00, 0xC0, 0xF2]
        );
    }

    #[test]
    fn can_encode_move_immediate_64_bit() {
        let instruction = Aarch64Instruction::MovImmToReg {
            register: Register::X1,
            value: 1234567890123456.,
        };

        let machine_code = instruction.make_machine_code();
        assert_eq!(
            machine_code,
            vec![
                0x01, 0x58, 0x97, 0xd2, 0x41, 0x91, 0xA7, 0xF2, 0xA1, 0x5A, 0xCC, 0xF2, 0x81, 0x00,
                0xE0, 0xF2,
            ]
        );
    }

    #[test]
    fn can_encode_move_reg_to_reg() {
        let instruction = Aarch64Instruction::MovRegToReg {
            source: Register::X8,
            destination: Register::X0,
        };

        let machine_code = instruction.make_machine_code();
        assert_eq!(machine_code, vec![0xE0, 0x03, 0x08, 0xAA]);
    }

    #[test]
    fn can_encode_add_reg_to_reg() {
        let instruction = Aarch64Instruction::AddRegToReg {
            destination: Register::X0,
            reg1: Register::X9,
            reg2: Register::X10,
        };

        let machine_code = instruction.make_machine_code();
        assert_eq!(machine_code, vec![0x20, 0x01, 0x0A, 0x8B]);
    }

    #[test]
    fn can_encode_sub_reg_to_reg() {
        let instruction = Aarch64Instruction::SubRegToReg {
            destination: Register::X0,
            reg1: Register::X9,
            reg2: Register::X10,
        };

        let machine_code = instruction.make_machine_code();
        assert_eq!(machine_code, vec![0x20, 0x01, 0x0A, 0xEB]);
    }

    #[test]
    fn can_encode_mul_reg_to_reg() {
        let instruction = Aarch64Instruction::MulRegToReg {
            destination: Register::X0,
            reg1: Register::X9,
            reg2: Register::X10,
        };

        let machine_code = instruction.make_machine_code();
        assert_eq!(machine_code, vec![0x20, 0x7D, 0x0A, 0x9B]);
    }

    #[test]
    fn can_encode_div_reg_to_reg() {
        let instruction = Aarch64Instruction::DivRegToReg {
            destination: Register::X0,
            reg1: Register::X9,
            reg2: Register::X10,
        };

        let machine_code = instruction.make_machine_code();
        assert_eq!(machine_code, vec![0x20, 0x0D, 0xCA, 0x9A]);
    }

    #[test]
    fn can_compile_trivial_function() {
        let program = parse_program("fn the_answer() { let a = 42; return a; }").unwrap();
        let compiled = frontend::compile(program);
        assert_eq!(compiled.len(), 1);

        let mut gen = Aarch64Generator::default();
        let machine_code = gen.generate_machine_code(&compiled[0]);
        println!("{}", machine_code.asm);
        machine_code
            .machine_code
            .iter()
            .for_each(|byte| print!("{:02X} ", byte));
    }
}
