use std::collections::HashMap;

use crate::ir::{CompiledFunction, IrInstruction, IrRegister};

/// Deduplicates constant assignments, retaining only the first and and replacing any reference
/// to the second register with a reference to the first
fn deduplicate_constants(
    body: Vec<IrInstruction>,
    num_used_registers: usize,
) -> Vec<IrInstruction> {
    // By default, each register maps to itself
    let mut register_replacement: Vec<IrRegister> = Vec::with_capacity(num_used_registers);
    for i in 0..num_used_registers {
        register_replacement.push(IrRegister::new(i));
    }

    let mut constant_values: HashMap<i64, IrRegister> = HashMap::new();

    let mut result = Vec::new();
    body.into_iter().for_each(|instruction| match instruction {
        IrInstruction::Mvi { dest, val } => {
            let register_containing_value = constant_values.get(&val);
            if let Some(register_containing_value) = register_containing_value {
                // Replace register with cached version in successive instructions, and skip it
                register_replacement[dest.0] = *register_containing_value;
            } else {
                constant_values.insert(val, dest);
                result.push(instruction.clone());
            }
        }
        IrInstruction::MvArg { .. } => {
            result.push(instruction.clone());
        }
        IrInstruction::BinOp {
            operator,
            dest,
            op1,
            op2,
        } => result.push(IrInstruction::BinOp {
            operator,
            dest,
            op1: register_replacement[op1.0],
            op2: register_replacement[op2.0],
        }),
        IrInstruction::Neg { dest, op } => result.push(IrInstruction::Neg {
            dest,
            op: register_replacement[op.0],
        }),
        IrInstruction::Ret { reg } => result.push(IrInstruction::Ret {
            reg: register_replacement[reg.0],
        }),
        IrInstruction::Call {
            dest,
            name,
            function_id,
            args,
        } => {
            let args = args.iter().map(|arg| register_replacement[arg.0]).collect();
            result.push(IrInstruction::Call {
                dest,
                name: name.clone(),
                function_id,
                args,
            })
        }
    });
    result
}

/// Removes dead store allocations, i.e. movements to registers that aren't used
/// in any `ret` statement
fn dead_store_elimination(
    body: Vec<IrInstruction>,
    num_used_registers: usize,
) -> Vec<IrInstruction> {
    let mut used_registers = vec![false; num_used_registers];

    let mut result = Vec::new();
    for instruction in body.into_iter().rev() {
        match instruction {
            IrInstruction::Ret { reg } => {
                used_registers[reg.0] = true;
                result.push(instruction);
            }
            IrInstruction::Mvi { dest, .. } => {
                if used_registers[dest.0] {
                    result.push(instruction);
                }
            }
            IrInstruction::MvArg { dest, .. } => {
                if used_registers[dest.0] {
                    result.push(instruction);
                }
            }
            IrInstruction::BinOp {
                dest,
                op1,
                op2,
                operator: _,
            } => {
                if used_registers[dest.0] {
                    used_registers[op1.0] = true;
                    used_registers[op2.0] = true;
                    result.push(instruction);
                }
            }
            IrInstruction::Neg { dest, op } => {
                if used_registers[dest.0] {
                    let op: usize = op.0;
                    used_registers[op] = true;
                    result.push(instruction);
                }
            }
            IrInstruction::Call { dest, ref args, .. } => {
                if used_registers[dest.0] {
                    for arg in args {
                        used_registers[arg.0] = true;
                    }
                    result.push(instruction);
                }
            }
        }
    }
    result.reverse();
    result
}

struct ReplacedBody {
    body: Vec<IrInstruction>,
    num_used_registers: usize,
}

/// Renames registers to be dense, starting from zero
fn rename_registers(body: Vec<IrInstruction>, num_used_registers: usize) -> ReplacedBody {
    // By default, each register maps to itself
    let mut register_replacement: Vec<IrRegister> = Vec::with_capacity(num_used_registers);
    for i in 0..num_used_registers {
        register_replacement.push(IrRegister::new(i));
    }

    let mut next_expected_register = 0;
    let mut result = Vec::with_capacity(body.len());
    for instruction in body {
        match instruction {
            IrInstruction::Ret { reg } => {
                // Ret is the only instruction that does increment next_expect_register
                // because it does not alllocate a new register
                result.push(IrInstruction::Ret {
                    reg: register_replacement[reg.0],
                });
            }
            IrInstruction::Mvi { dest, val } => {
                if next_expected_register == dest.0 {
                    result.push(instruction.clone());
                } else {
                    let replaced_register = IrRegister::new(next_expected_register);
                    result.push(IrInstruction::Mvi {
                        dest: replaced_register,
                        val,
                    });
                    register_replacement[dest.0] = replaced_register;
                }
                next_expected_register += 1;
            }
            IrInstruction::MvArg { dest, arg } => {
                if next_expected_register == dest.0 {
                    result.push(instruction.clone());
                } else {
                    let replaced_register = IrRegister::new(next_expected_register);
                    result.push(IrInstruction::MvArg {
                        dest: replaced_register,
                        arg,
                    });
                    register_replacement[dest.0] = replaced_register;
                }
                next_expected_register += 1;
            }
            IrInstruction::BinOp {
                operator,
                dest,
                op1,
                op2,
            } => {
                dbg!(&register_replacement);
                dbg!(&register_replacement);
                if next_expected_register == dest.0 {
                    result.push(IrInstruction::BinOp {
                        operator,
                        dest,
                        op1: register_replacement[op1.0],
                        op2: register_replacement[op2.0],
                    });
                } else {
                    let replaced_register = IrRegister::new(next_expected_register);
                    result.push(IrInstruction::BinOp {
                        operator,
                        dest: replaced_register,
                        op1: register_replacement[op1.0],
                        op2: register_replacement[op2.0],
                    });
                    register_replacement[dest.0] = replaced_register;
                }
                next_expected_register += 1;
            }
            IrInstruction::Neg { dest, op } => {
                if next_expected_register == dest.0 {
                    result.push(IrInstruction::Neg {
                        dest,
                        op: register_replacement[op.0],
                    });
                } else {
                    let replaced_register = IrRegister::new(next_expected_register);
                    result.push(IrInstruction::Neg {
                        dest: replaced_register,
                        op: register_replacement[op.0],
                    });
                    register_replacement[dest.0] = replaced_register;
                }
                next_expected_register += 1;
            }
            IrInstruction::Call {
                dest,
                name,
                function_id,
                args,
            } => {
                let args = args
                    .into_iter()
                    .map(|arg| register_replacement[arg.0])
                    .collect();

                if next_expected_register == dest.0 {
                    result.push(IrInstruction::Call {
                        dest,
                        name,
                        function_id,
                        args,
                    });
                } else {
                    let replaced_register = IrRegister::new(next_expected_register);
                    result.push(IrInstruction::Call {
                        dest: replaced_register,
                        name,
                        function_id,
                        args,
                    });
                    register_replacement[dest.0] = replaced_register;
                }
                next_expected_register += 1;
            }
        }
    }

    ReplacedBody {
        body: result,
        num_used_registers: next_expected_register,
    }
}

pub fn optimize_fun(fun: CompiledFunction) -> CompiledFunction {
    let body = deduplicate_constants(fun.body, fun.num_used_registers);
    let body = dead_store_elimination(body, fun.num_used_registers);
    let ReplacedBody {
        body,
        num_used_registers,
    } = rename_registers(body, fun.num_used_registers);
    CompiledFunction {
        name: fun.name,
        id: fun.id,
        num_args: fun.num_args,
        body,
        num_used_registers,
    }
}

pub fn optimize(functions: Vec<CompiledFunction>) -> Vec<CompiledFunction> {
    functions.into_iter().map(|fun| optimize_fun(fun)).collect()
}

#[cfg(test)]
mod tests {
    use crate::ir::builders::{add, call, mvarg, mvi};

    use super::*;

    #[test]
    fn can_deduplicate_constants() {
        let body = vec![
            mvi(0, 1),
            mvi(1, 2),
            mvi(2, 1),
            add(3, 1, 2),
            call(4, "f", 0, vec![3, 0, 2]),
        ];
        let optimized = deduplicate_constants(body, 4);

        assert_eq!(
            vec![
                mvi(0, 1),
                mvi(1, 2),
                add(3, 1, 0),
                call(4, "f", 0, vec![3, 0, 0])
            ],
            optimized,
        );
    }

    #[test]
    fn can_remove_dead_store() {
        let body = vec![
            mvi(0, 1),
            mvi(1, 2),
            mvi(2, 4),
            add(3, 1, 0),
            call(4, "f", 0, vec![3]),
        ];
        let optimized = dead_store_elimination(body, 5);

        assert_eq!(
            vec![mvi(0, 1), mvi(1, 2), add(3, 1, 0), call(4, "f", 0, vec![3])],
            optimized,
        );
    }

    #[test]
    fn can_rename_registers() {
        let body = vec![mvi(1, 1), add(3, 1, 1), call(4, "f", 0, vec![3])];
        let optimized = rename_registers(body, 5);

        assert_eq!(
            vec![mvi(0, 1), add(1, 0, 0), call(2, "f", 0, vec![1])],
            optimized.body,
        );
        assert_eq!(3, optimized.num_used_registers);
    }
}
