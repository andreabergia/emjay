use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    ast::{Block, BlockElement, Expression, Function, Program},
    ir::{CompiledFunction, Instruction, RegisterIndex},
};

pub fn compile(program: Program) -> Vec<CompiledFunction> {
    program
        .iter()
        .map(|f| {
            let global_symbol_table = Rc::new(RefCell::new(SymbolTable::default()));
            let mut compiler = FunctionCompiler::new(global_symbol_table);
            compiler.compile_function(f)
        })
        .collect()
}

#[derive(Clone)]
enum Symbol<'input> {
    Function {
        name: &'input str,
    },
    Variable {
        name: &'input str,
        allocated_register: RegisterIndex,
    },
}

impl<'input> Symbol<'input> {
    fn name(&self) -> &'input str {
        match self {
            Symbol::Function { name } => name,
            Symbol::Variable { name, .. } => name,
        }
    }
}

#[derive(Default)]
struct SymbolTable<'input> {
    parent: Option<SymbolTableRef<'input>>,
    names_to_symbols: HashMap<&'input str, Symbol<'input>>,
}

type SymbolTableRef<'input> = Rc<RefCell<SymbolTable<'input>>>;

impl<'input> SymbolTable<'input> {
    fn with_parent(parent: SymbolTableRef<'input>) -> Self {
        Self {
            parent: Some(parent),
            names_to_symbols: HashMap::new(),
        }
    }

    fn lookup(&self, name: &str) -> Option<Symbol<'input>> {
        self.names_to_symbols.get(name).cloned().or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.borrow().lookup(name))
        })
    }

    fn put(&mut self, symbol: Symbol<'input>) {
        let name = symbol.name();
        self.names_to_symbols.insert(name, symbol);
    }

    fn store_location(&mut self, name: &str, register: RegisterIndex) {
        let symbol = self.names_to_symbols.get_mut(name);
        match symbol {
            None => match &self.parent {
                None => panic!("trying to overwrite undeclared identifier {}", name),
                Some(parent) => {
                    parent.borrow_mut().store_location(name, register);
                }
            },
            Some(Symbol::Function { .. }) => panic!("cannot assign location of function {}", name),
            Some(Symbol::Variable {
                allocated_register, ..
            }) => *allocated_register = register,
        };
    }
}

struct FunctionCompiler<'input> {
    curr_symbol_table: SymbolTableRef<'input>,
    next_free_reg: RegisterIndex,
}

struct SymbolTablePopper<'input, 'fc> {
    compiler: &'fc mut FunctionCompiler<'input>,
    prev_symbol_table: SymbolTableRef<'input>,
}

impl Drop for SymbolTablePopper<'_, '_> {
    fn drop(&mut self) {
        println!("dropping");
        self.compiler.curr_symbol_table = self.prev_symbol_table.clone();
    }
}

impl<'input> FunctionCompiler<'input> {
    fn new(parent_symbol_table: SymbolTableRef<'input>) -> Self {
        Self {
            curr_symbol_table: Rc::new(RefCell::new(SymbolTable::with_parent(parent_symbol_table))),
            next_free_reg: RegisterIndex::from_u32(0),
        }
    }

    fn push_symbol_table<'s>(&'s mut self) -> SymbolTablePopper<'input, 's> {
        let prev_symbol_table = self.curr_symbol_table.clone();
        self.curr_symbol_table = Rc::new(RefCell::new(SymbolTable::with_parent(
            prev_symbol_table.clone(),
        )));
        SymbolTablePopper {
            compiler: self,
            prev_symbol_table,
        }
    }

    fn compile_function(&mut self, f: &Function<'input>) -> CompiledFunction<'input> {
        self.push_symbol_table();
        let mut body: Vec<Instruction> = Vec::new();
        self.compile_block(&mut body, &f.block);
        CompiledFunction {
            name: f.name,
            num_used_registers: usize::from(self.next_free_reg),
            body,
        }
    }

    fn compile_block(&mut self, body: &mut Vec<Instruction>, block: &Block<'input>) {
        self.push_symbol_table();
        block.iter().for_each(|element| match element {
            BlockElement::NestedBlock(nested) => self.compile_block(body, nested),
            BlockElement::LetStatement { name, expression } => {
                // TODO: check symbol does not exist
                let reg = self.compile_expression(body, expression);
                self.curr_symbol_table.borrow_mut().put(Symbol::Variable {
                    name,
                    allocated_register: reg,
                });
            }
            BlockElement::AssignmentStatement { name, expression } => {
                let existing_symbol = self.curr_symbol_table.borrow_mut().lookup(name);
                match existing_symbol {
                    None => {
                        // TODO: proper error
                        panic!("trying to assign to undeclared identifier {}", name)
                    }
                    Some(symbol) => {
                        match symbol {
                            Symbol::Function { .. } => {
                                // TODO: proper error
                                panic!("cannot assign value to function {}", name)
                            }
                            Symbol::Variable { .. } => {
                                let reg = self.compile_expression(body, expression);
                                self.curr_symbol_table
                                    .borrow_mut()
                                    .store_location(name, reg);
                            }
                        }
                    }
                }
            }
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
                let symbol = self.curr_symbol_table.borrow().lookup(id);
                match symbol {
                    None => panic!("undeclared identifier {}", id),
                    Some(Symbol::Function { .. }) => {
                        panic!("function {} is not a valid variable name", id)
                    }
                    Some(Symbol::Variable {
                        allocated_register, ..
                    }) => allocated_register,
                }
            }
            Expression::Number(n) => {
                let reg = self.allocate_reg();
                body.push(Instruction::Mvi { dest: reg, val: *n });
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
        self.next_free_reg.inc()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        ir::builders::{add, mvi, ret},
        parser::*,
    };

    #[test]
    fn can_compile_variable_declaration_and_math() {
        let program = parse_program("fn the_answer() { let a = 3; return a + 1; }").unwrap();
        let compiled = compile(program);
        assert_eq!(compiled.len(), 1);

        let f = &compiled[0];
        assert_eq!(f.name, "the_answer");
        assert_eq!(f.num_used_registers, 3);
        assert_eq!(f.body, vec![mvi(0, 3.0), mvi(1, 1.0), add(2, 0, 1), ret(2)]);
    }

    #[test]
    fn can_compile_assignments() {
        let program = parse_program("fn the_answer() { let a = 1; a = 2; return a; }").unwrap();
        let compiled = compile(program);
        assert_eq!(compiled.len(), 1);

        let f = &compiled[0];
        assert_eq!(f.name, "the_answer");
        assert_eq!(f.num_used_registers, 2);
        assert_eq!(f.body, vec![mvi(0, 1.0), mvi(1, 2.0), ret(1)]);
    }
}
