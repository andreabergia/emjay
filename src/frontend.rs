use std::collections::HashMap;

use crate::{
    ast::{Block, BlockElement, Expression, Function, Program},
    ir::{CompiledFunction, Instruction, RegisterIndex},
};

fn compile(program: Program) -> Vec<CompiledFunction> {
    program
        .iter()
        .map(|f| {
            let mut compiler = FunctionCompiler::default();
            compiler.compile_function(f)
        })
        .collect()
}

#[derive(Default)]
struct FunctionCompiler<'input> {
    id_to_reg: HashMap<&'input str, RegisterIndex>,
    next_free_reg: RegisterIndex,
}

impl<'input> FunctionCompiler<'input> {
    fn compile_function(&mut self, f: &Function<'input>) -> CompiledFunction<'input> {
        let mut body: Vec<Instruction> = Vec::new();
        self.compile_block(&mut body, &f.block);
        CompiledFunction { name: f.name, body }
    }

    fn compile_block(&mut self, body: &mut Vec<Instruction>, block: &Block<'input>) {
        block.iter().for_each(|element| match element {
            BlockElement::NestedBlock(nested) => self.compile_block(body, nested),
            BlockElement::LetStatement { name, expression } => {
                // TODO: error in case identifier exists
                let reg = self.compile_expression(body, expression);
                self.id_to_reg.insert(name, reg);
            }

            BlockElement::AssignmentStatement { name, expression } => todo!(),
            BlockElement::ReturnStatement(expression) => {
                let reg = self.compile_expression(body, expression);
                body.push(Instruction::Ret { reg });
            }
        })
    }

    fn compile_expression(
        &mut self,
        body: &mut Vec<Instruction>,
        expression: &Expression,
    ) -> RegisterIndex {
        match expression {
            Expression::Identifier(id) => {
                // TODO: proper error
                let reg_with_value = self.id_to_reg.get(id).expect("using undeclared identifier");
                *reg_with_value
            }
            Expression::Number(n) => {
                let reg = self.allocate_reg();
                body.push(Instruction::Mov { dest: reg, val: *n });
                reg
            }
            Expression::Negate(_) => todo!(),
            Expression::Add(left, right) => {
                let op1 = self.compile_expression(body, left);
                let op2 = self.compile_expression(body, right);
                let dest = self.allocate_reg();
                body.push(Instruction::Add { dest, op1, op2 });
                dest
            }
            Expression::Sub(left, right) => {
                let op1 = self.compile_expression(body, left);
                let op2 = self.compile_expression(body, right);
                let dest = self.allocate_reg();
                body.push(Instruction::Sub { dest, op1, op2 });
                dest
            }
            Expression::Mul(left, right) => {
                let op1 = self.compile_expression(body, left);
                let op2 = self.compile_expression(body, right);
                let dest = self.allocate_reg();
                body.push(Instruction::Mul { dest, op1, op2 });
                dest
            }
            Expression::Div(left, right) => {
                let op1 = self.compile_expression(body, left);
                let op2 = self.compile_expression(body, right);
                let dest = self.allocate_reg();
                body.push(Instruction::Div { dest, op1, op2 });
                dest
            }

            _ => todo!(),
        }
    }

    fn allocate_reg(&mut self) -> RegisterIndex {
        let reg = self.next_free_reg;
        self.next_free_reg += 1;
        reg
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::parser::*;

    #[test]
    fn can_compile_trivial_function() {
        let program =
            parse_program("fn the_answer() { let a = 3; let b = 4; return a * b + 1; }").unwrap();
        let compiled = compile(program);
        assert_eq!(compiled.len(), 1);
        println!("{}", compiled[0]);
    }
}
