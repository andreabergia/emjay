use std::{cell::RefCell, collections::HashMap, rc::Rc};

use thiserror::Error;

use crate::{
    ast::{Block, BlockElement, Expression, Function, Program},
    ir::{CompiledFunction, IrInstruction, IrRegister},
};

#[derive(Debug, Error)]
pub enum FrontendError {
    #[error("variable \"{name}\" not defined")]
    VariableNotDefined {
        name: String,
        // TODO: location: SourceLocation,
    },
    #[error("variable \"{name}\" already defined")]
    VariableAlreadyDefined {
        name: String,
        // TODO: location: SourceLocation,
    },
    #[error("variable \"{name}\" cannot shadow function argument with the same name")]
    VariableCannotShadowArgument { name: String },
}

pub fn compile(program: Program) -> Result<Vec<CompiledFunction>, FrontendError> {
    program
        .iter()
        .map(|f| {
            let global_symbol_table = SymbolTable::new();
            let mut compiler = FunctionCompiler::default();
            compiler.compile_function(f, global_symbol_table)
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
        allocated_register: IrRegister,
    },
    Argument {
        name: &'input str,
        index: usize,
    },
}

impl<'input> Symbol<'input> {
    fn name(&self) -> &'input str {
        match self {
            Symbol::Function { name } => name,
            Symbol::Variable { name, .. } => name,
            Symbol::Argument { name, .. } => name,
        }
    }
}

#[derive(Default)]
struct SymbolTable<'input> {
    parent: Option<Rc<RefCell<SymbolTable<'input>>>>,
    names_to_symbols: HashMap<&'input str, Symbol<'input>>,
}

type SymbolTableRef<'input> = Rc<RefCell<SymbolTable<'input>>>;

impl<'input> SymbolTable<'input> {
    fn new() -> SymbolTableRef<'input> {
        Rc::new(RefCell::new(SymbolTable::default()))
    }

    fn with_parent(parent: SymbolTableRef<'input>) -> SymbolTableRef<'input> {
        Rc::new(RefCell::new(Self {
            parent: Some(parent),
            names_to_symbols: HashMap::new(),
        }))
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

    // TODO: we shouldn't need two lookups in the table
    fn store_location(&mut self, name: &str, register: IrRegister) {
        let symbol = self.names_to_symbols.get_mut(name);
        match symbol {
            None => match &self.parent {
                None => panic!("trying to overwrite undeclared identifier {}", name),
                Some(parent) => {
                    parent.borrow_mut().store_location(name, register);
                }
            },
            Some(Symbol::Function { .. }) => panic!("cannot assign location of function {}", name),
            Some(Symbol::Argument { .. }) => panic!("cannot assign location of arguments {}", name),
            Some(Symbol::Variable {
                allocated_register, ..
            }) => *allocated_register = register,
        };
    }
}

#[derive(Default)]
struct FunctionCompiler {
    next_free_reg: IrRegister,
}

impl<'input> FunctionCompiler {
    fn compile_function(
        &mut self,
        f: &Function<'input>,
        parent_symbol_table: SymbolTableRef<'input>,
    ) -> Result<CompiledFunction<'input>, FrontendError> {
        let symbol_table = SymbolTable::with_parent(parent_symbol_table);
        let mut body: Vec<IrInstruction> = Vec::new();
        Self::define_args(f, symbol_table.clone());
        self.compile_block(&mut body, &f.block, symbol_table)?;
        Ok(CompiledFunction {
            name: f.name,
            num_args: f.args.len(),
            num_used_registers: usize::from(self.next_free_reg),
            body,
        })
    }

    fn define_args(f: &Function<'input>, symbol_table: SymbolTableRef<'input>) {
        for (index, arg) in f.args.iter().enumerate() {
            symbol_table
                .borrow_mut()
                .put(Symbol::Argument { name: arg, index });
        }
    }

    fn compile_block(
        &mut self,
        body: &mut Vec<IrInstruction>,
        block: &Block<'input>,
        parent_symbol_table: SymbolTableRef<'input>,
    ) -> Result<(), FrontendError> {
        let symbol_table = SymbolTable::with_parent(parent_symbol_table);
        println!("compiling block: {:?}", block);
        for element in block.iter() {
            match element {
                BlockElement::NestedBlock(nested) => {
                    self.compile_block(body, nested, symbol_table.clone())?
                }
                BlockElement::LetStatement { name, expression } => {
                    match symbol_table.borrow().lookup(name) {
                        Some(Symbol::Variable { .. }) => {
                            return Err(FrontendError::VariableAlreadyDefined {
                                name: name.to_string(),
                            });
                        }
                        Some(Symbol::Argument { .. }) => {
                            return Err(FrontendError::VariableCannotShadowArgument {
                                name: name.to_string(),
                            });
                        }
                        _ => (),
                    }
                    let reg = self.compile_expression(body, expression, symbol_table.clone())?;
                    symbol_table.borrow_mut().put(Symbol::Variable {
                        name,
                        allocated_register: reg,
                    });
                }
                BlockElement::AssignmentStatement { name, expression } => {
                    let existing_symbol = symbol_table.borrow().lookup(name);
                    match existing_symbol {
                        Some(Symbol::Variable { .. }) => {
                            let reg =
                                self.compile_expression(body, expression, symbol_table.clone())?;
                            symbol_table.borrow_mut().store_location(name, reg);
                        }
                        Some(Symbol::Argument { name, index }) => {
                            let reg = self.allocate_reg();
                            body.push(IrInstruction::MvArg {
                                dest: reg,
                                arg: index.into(),
                            });

                            // Overwrite the entry in the symbol table so that future lookups will not need
                            // to copy again the argument into a register
                            symbol_table.borrow_mut().put(Symbol::Variable {
                                name,
                                allocated_register: reg,
                            });

                            let reg =
                                self.compile_expression(body, expression, symbol_table.clone())?;
                            symbol_table.borrow_mut().store_location(name, reg);
                        }
                        _ => {
                            return Err(FrontendError::VariableNotDefined {
                                name: name.to_string(),
                            });
                        }
                    }
                }
                BlockElement::ReturnStatement(expression) => {
                    let reg = self.compile_expression(body, expression, symbol_table.clone())?;
                    body.push(IrInstruction::Ret { reg });
                }
            }
        }
        Ok(())
    }

    fn compile_expression(
        &mut self,
        body: &mut Vec<IrInstruction>,
        expression: &Expression,
        symbol_table: SymbolTableRef<'input>,
    ) -> Result<IrRegister, FrontendError> {
        match expression {
            Expression::Identifier(name) => {
                let symbol = symbol_table.borrow().lookup(name);
                match symbol {
                    Some(Symbol::Variable {
                        allocated_register, ..
                    }) => Ok(allocated_register),
                    Some(Symbol::Argument { name, index }) => {
                        let reg = self.allocate_reg();
                        body.push(IrInstruction::MvArg {
                            dest: reg,
                            arg: index.into(),
                        });

                        // Overwrite the entry in the symbol table so that future lookups will not need
                        // to copy again the argument into a register
                        symbol_table.borrow_mut().put(Symbol::Variable {
                            name,
                            allocated_register: reg,
                        });

                        Ok(reg)
                    }
                    _ => Err(FrontendError::VariableNotDefined {
                        name: name.to_string(),
                    }),
                }
            }
            Expression::Number(n) => {
                let reg = self.allocate_reg();
                body.push(IrInstruction::Mvi { dest: reg, val: *n });
                Ok(reg)
            }
            Expression::FunctionCall(call) => {
                let dest = self.allocate_reg();
                let args = call
                    .args
                    .iter()
                    .map(|arg| self.compile_expression(body, arg, symbol_table.clone()))
                    .collect::<Result<Vec<_>, _>>()?;
                body.push(IrInstruction::Call {
                    dest,
                    name: call.name.to_string(),
                    args,
                });
                Ok(dest)
            }
            Expression::Negate(_) => todo!(),
            Expression::Add(left, right) => {
                let op1 = self.compile_expression(body, left, symbol_table.clone())?;
                let op2 = self.compile_expression(body, right, symbol_table)?;
                let dest = self.allocate_reg();
                body.push(IrInstruction::Add { dest, op1, op2 });
                Ok(dest)
            }
            Expression::Sub(left, right) => {
                let op1 = self.compile_expression(body, left, symbol_table.clone())?;
                let op2 = self.compile_expression(body, right, symbol_table)?;
                let dest = self.allocate_reg();
                body.push(IrInstruction::Sub { dest, op1, op2 });
                Ok(dest)
            }
            Expression::Mul(left, right) => {
                let op1 = self.compile_expression(body, left, symbol_table.clone())?;
                let op2 = self.compile_expression(body, right, symbol_table)?;
                let dest = self.allocate_reg();
                body.push(IrInstruction::Mul { dest, op1, op2 });
                Ok(dest)
            }
            Expression::Div(left, right) => {
                let op1 = self.compile_expression(body, left, symbol_table.clone())?;
                let op2 = self.compile_expression(body, right, symbol_table)?;
                let dest = self.allocate_reg();
                body.push(IrInstruction::Div { dest, op1, op2 });
                Ok(dest)
            }

            _ => todo!(),
        }
    }

    fn allocate_reg(&mut self) -> IrRegister {
        self.next_free_reg.inc()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        ir::builders::{add, call, div, mul, mvarg, mvi, ret, sub},
        parser::*,
    };

    #[test]
    fn happy_path() {
        let program = parse_program(
            r"
            fn the_answer(x) {
                let a = 3;
                x = x + 1;
                return a + x - 2 * 3 / f(a, x);
            }",
        )
        .unwrap();
        let compiled = compile(program).unwrap();
        assert_eq!(compiled.len(), 1);

        let f = &compiled[0];
        println!("f: {}", f);
        assert_eq!(f.name, "the_answer");
        assert_eq!(f.num_used_registers, 11);
        assert_eq!(
            f.body,
            vec![
                mvi(0, 3.0),
                mvarg(1, 0),
                mvi(2, 1.0),
                add(3, 1, 2),
                add(4, 0, 3),
                mvi(5, 2.0),
                mvi(6, 3.0),
                mul(7, 5, 6),
                call(8, "f", vec![0, 3]),
                div(9, 7, 8),
                sub(10, 4, 9),
                ret(10),
            ]
        );
    }

    #[test]
    fn can_compile_assignments() {
        let program = parse_program(
            r"fn the_answer() {
                let a = 1;
                {
                    a = 2;
                }
                return a;
            }",
        )
        .unwrap();
        let compiled = compile(program).unwrap();
        assert_eq!(compiled.len(), 1);

        let f = &compiled[0];
        assert_eq!(f.name, "the_answer");
        assert_eq!(f.num_used_registers, 2);
        assert_eq!(f.body, vec![mvi(0, 1.0), mvi(1, 2.0), ret(1)]);
    }

    #[test]
    fn can_refer_to_outside_variable_from_nested_block() {
        let program = parse_program(
            r"fn the_answer() {
                let a = 1;
                {
                    return a;
                }
            }",
        )
        .unwrap();
        let compiled = compile(program).unwrap();
        assert_eq!(compiled.len(), 1);

        let f = &compiled[0];
        assert_eq!(f.name, "the_answer");
        assert_eq!(f.num_used_registers, 1);
        assert_eq!(f.body, vec![mvi(0, 1.0), ret(0)]);
    }

    #[test]
    fn compile_error_return_undeclared_variable() {
        let program = parse_program("fn f() { return a; }").unwrap();
        let error = compile(program).unwrap_err();
        assert_eq!(error.to_string(), "variable \"a\" not defined");
    }

    #[test]
    fn compile_error_assign_to_undeclared_variable() {
        let program = parse_program("fn f() { a = 1; }").unwrap();
        let error = compile(program).unwrap_err();
        assert_eq!(error.to_string(), "variable \"a\" not defined");
    }

    #[test]
    fn compile_error_double_variable_declaration() {
        let program = parse_program("fn f() { let a = 1; let a = 2; }").unwrap();
        let error = compile(program).unwrap_err();
        assert_eq!(error.to_string(), "variable \"a\" already defined");
    }

    #[test]
    fn compile_error_variable_declared_in_nested_block() {
        let program = parse_program(
            r"fn f() {
                {
                    let a = 1;
                }
                return a;
            }",
        )
        .unwrap();
        let error = compile(program).unwrap_err();
        assert_eq!(error.to_string(), "variable \"a\" not defined");
    }

    #[test]
    fn compile_error_variable_cannot_be_shadowed_in_nested_block() {
        let program = parse_program(
            r"fn f() {
                let a = 1;
                {
                    let a = 2;
                }
                return a;
            }",
        )
        .unwrap();
        let error = compile(program).unwrap_err();
        assert_eq!(error.to_string(), "variable \"a\" already defined");
    }

    #[test]
    fn compile_error_fn_arg_cannot_be_shadowed() {
        let program = parse_program(
            r"fn f(x) {
                let x = 1;
            }",
        )
        .unwrap();
        let error = compile(program).unwrap_err();
        assert_eq!(
            error.to_string(),
            "variable \"x\" cannot shadow function argument with the same name"
        );
    }
}
