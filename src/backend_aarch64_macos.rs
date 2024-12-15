use std::fmt::{Display, Write};

use crate::{
    backend::{GeneratedMachineCode, MachineCodeGenerator},
    ir::{CompiledFunction, Instruction},
};

#[derive(Default)]
pub struct Aarch64MacOsGenerator {}

#[derive(Debug, Clone, Copy)]
enum Register {
    X0,
    X1,
    W0,
    W1,
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Register::X0 => write!(f, "x0"),
            Register::X1 => write!(f, "x1"),
            Register::W0 => write!(f, "w0"),
            Register::W1 => write!(f, "w1"),
        }
    }
}

enum Aarch64Instruction {
    Ret,
    MovImmToReg { register: Register, value: f64 },
}

impl Display for Aarch64Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Aarch64Instruction::Ret => write!(f, "ret"),
            Aarch64Instruction::MovImmToReg { register, value } => {
                write!(f, "mov {}, {}", register, value)
            }
        }
    }
}

impl Aarch64Instruction {
    fn make_machine_code(&self) -> Vec<u8> {
        match self {
            Aarch64Instruction::Ret => vec![0xc0, 0x03, 0x5f, 0xd6],
            Aarch64Instruction::MovImmToReg { register, value } => {
                // TODO: encode value and register properly
                vec![0x40, 0x05, 0x80, 0x52]
            }
        }
    }
}

impl MachineCodeGenerator for Aarch64MacOsGenerator {
    fn generate_machine_code(&mut self, function: &CompiledFunction) -> GeneratedMachineCode {
        let mut instructions = Vec::new();

        for instruction in function.body.iter() {
            match instruction {
                Instruction::Mvi { dest, val } => {
                    // TODO: check destination
                    instructions.push(Aarch64Instruction::MovImmToReg {
                        register: Register::W0,
                        value: *val,
                    })
                }
                Instruction::Ret { reg } => instructions.push(Aarch64Instruction::Ret),
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::{frontend, parser::*};

    #[test]
    fn can_compile_trivial_function() {
        let program = parse_program("fn the_answer() { let a = 42; return a; }").unwrap();
        let compiled = frontend::compile(program);
        assert_eq!(compiled.len(), 1);

        let mut gen = Aarch64MacOsGenerator::default();
        let machine_code = gen.generate_machine_code(&compiled[0]);
        println!("{}", machine_code.asm);
        machine_code
            .machine_code
            .iter()
            .for_each(|byte| print!("{:02X} ", byte));
    }
}
