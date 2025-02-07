use std::collections::HashMap;

use crate::ir::{BinOpOperator::*, CompiledFunction, IrInstruction, IrRegister};

/// Replaces algebraic expressions with their computed values, if possible. For example:
/// ```
/// mov r0, 1
/// mov r1, 2
/// add r2, r0, r1
/// ````
///
/// becomes
///
/// ```
/// mov r0, 1
/// mov r1, 2
/// mov r2, 3
/// ````
fn propagate_constants(body: Vec<IrInstruction>, num_used_registers: usize) -> Vec<IrInstruction> {
    let mut known_constants: Vec<Option<i64>> = vec![None; num_used_registers];

    let mut result = Vec::with_capacity(body.len());
    for instruction in body {
        match instruction {
            IrInstruction::Mvi { dest, val } => {
                known_constants[dest.0] = Some(val);
                result.push(instruction.clone());
            }
            IrInstruction::BinOp {
                operator,
                dest,
                op1,
                op2,
            } => {
                if let (Some(value1), Some(value2)) =
                    (known_constants[op1.0], known_constants[op2.0])
                {
                    let computed_value = match operator {
                        Add => value1 + value2,
                        Sub => value1 - value2,
                        Mul => value1 * value2,
                        Div => value1 / value2,
                    };
                    known_constants[dest.0] = Some(computed_value);
                    result.push(IrInstruction::Mvi {
                        dest,
                        val: computed_value,
                    })
                } else {
                    // Not a known constant, leave as-is
                    result.push(instruction.clone());
                }
            }
            IrInstruction::Neg { dest, op } => {
                if let Some(value) = known_constants[op.0] {
                    // Replace with a constant
                    let computed_value = -value;
                    known_constants[dest.0] = Some(computed_value);
                    result.push(IrInstruction::Mvi {
                        dest,
                        val: computed_value,
                    })
                } else {
                    // Not a known constant, leave as-is
                    result.push(instruction.clone());
                }
            }
            IrInstruction::Ret { .. }
            | IrInstruction::MvArg { .. }
            | IrInstruction::Call { .. } => {
                // Can't optimize
                result.push(instruction.clone());
            }
        }
    }
    result
}

/// Deduplicates constant assignments, retaining only the first and and replacing any reference
/// to the second register with a reference to the first. Meaning:
/// ```
/// mov r0, 1
/// mov r1, 1
/// ret r1
/// ```
///
/// becomes
///
/// ```
/// mov r0, 1
/// ret r0
/// ```
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
    for instruction in body {
        match instruction {
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
        }
    }
    result
}

/// Removes dead store allocations, i.e. movements to registers that aren't used
/// in any `ret` statement. For example:
/// ```
/// mov r0, 1
/// mov r1, 2
/// ret r0
/// ```
///
/// becomes
///
/// ```
/// mov r0, 1
/// ret r0
/// ````
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

struct OptimizedBody {
    body: Vec<IrInstruction>,
    num_used_registers: usize,
}

/// Renames registers to be dense, starting from zero. For example:
/// ```
/// mov r0, 1
/// mov r2, 2
/// add r4, r2, r0
/// ```
///
/// becomes:
///
/// ```
/// mov r0, 1
/// mov r1, 2
/// add r2, r1, r0
/// ```
fn rename_registers(body: Vec<IrInstruction>, num_used_registers: usize) -> OptimizedBody {
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

    OptimizedBody {
        body: result,
        num_used_registers: next_expected_register,
    }
}

fn optimize_fun_body(body: Vec<IrInstruction>, num_used_registers: usize) -> OptimizedBody {
    let body = propagate_constants(body, num_used_registers);
    let body = deduplicate_constants(body, num_used_registers);
    let body = dead_store_elimination(body, num_used_registers);
    rename_registers(body, num_used_registers)
}

pub fn optimize_fun(fun: CompiledFunction) -> CompiledFunction {
    let OptimizedBody {
        body,
        num_used_registers,
    } = optimize_fun_body(fun.body, fun.num_used_registers);
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
    use crate::ir::builders::{add, call, mul, mvarg, mvi, ret};

    use super::*;

    #[test]
    fn can_propagate_constants() {
        let body = vec![
            mvi(0, 1),
            mvi(1, 2),
            mvarg(2, 0),
            add(3, 0, 1),
            add(4, 3, 2),
            mvi(5, 5),
            add(6, 5, 3),
        ];
        let optimized = propagate_constants(body, 7);

        assert_eq!(
            vec![
                mvi(0, 1),
                mvi(1, 2),
                mvarg(2, 0),
                mvi(3, 3),
                add(4, 3, 2),
                mvi(5, 5),
                mvi(6, 8)
            ],
            optimized,
        );
    }

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
            ret(4),
        ];
        let optimized = dead_store_elimination(body, 5);

        assert_eq!(
            vec![
                mvi(0, 1),
                mvi(1, 2),
                add(3, 1, 0),
                call(4, "f", 0, vec![3]),
                ret(4)
            ],
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

    #[test]
    fn can_optimize() {
        let body = vec![
            mvi(0, 1),
            mvi(1, 2),
            add(2, 0, 1), // r2 will contain a constant, 3
            mvi(3, 3),
            mul(4, 2, 3), // r4 will contain a constant, 9
            mvi(5, 42),
            ret(4),
        ];
        let optimized = optimize_fun_body(body, 6);

        assert_eq!(vec![mvi(0, 9), ret(0)], optimized.body);
        assert_eq!(1, optimized.num_used_registers);
    }
}
