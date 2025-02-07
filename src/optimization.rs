use std::collections::HashMap;

use crate::ir::{CompiledFunction, IrInstruction, IrRegister};

/// Deduplicates constant assignments, retaining only the first and and replacing any reference
/// to the second register with a reference to the first.
fn deduplicate_constants(
    body: Vec<IrInstruction>,
    num_used_registers: usize,
) -> Vec<IrInstruction> {
    // By default, each register maps to itself
    let mut register_replacement: Vec<IrRegister> = Vec::with_capacity(num_used_registers);
    for i in 0..num_used_registers {
        register_replacement.push(IrRegister::from_u32(i as u32));
    }

    let mut constant_values: HashMap<i64, IrRegister> = HashMap::new();

    let mut result = Vec::new();
    body.into_iter().for_each(|instruction| match instruction {
        IrInstruction::Mvi { dest, val } => {
            let dest_usize: usize = dest.into();
            let register_containing_value = constant_values.get(&val);
            if let Some(register_containing_value) = register_containing_value {
                // Replace register with cached version in successive instructions, and skip it
                register_replacement[dest_usize] = *register_containing_value;
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
        } => {
            let op1: usize = op1.into();
            let op2: usize = op2.into();
            result.push(IrInstruction::BinOp {
                operator,
                dest,
                op1: register_replacement[op1],
                op2: register_replacement[op2],
            })
        }
        IrInstruction::Neg { dest, op } => {
            let op: usize = op.into();
            result.push(IrInstruction::Neg {
                dest,
                op: register_replacement[op],
            })
        }
        IrInstruction::Ret { reg } => {
            let reg: usize = reg.into();
            result.push(IrInstruction::Ret {
                reg: register_replacement[reg],
            })
        }
        IrInstruction::Call {
            dest,
            name,
            function_id,
            args,
        } => {
            let args = args
                .iter()
                .map(|arg| {
                    let arg: usize = (*arg).into();
                    register_replacement[arg]
                })
                .collect();
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

fn dead_store_elimination(
    body: Vec<IrInstruction>,
    num_used_registers: usize,
) -> Vec<IrInstruction> {
    let mut used_registers = vec![false; num_used_registers];

    let mut result = Vec::new();
    for instruction in body.into_iter().rev() {
        match instruction {
            IrInstruction::Ret { reg } => {
                let reg: usize = reg.into();
                used_registers[reg] = true;
                result.push(instruction);
            }
            IrInstruction::Mvi { dest, .. } => {
                let dest: usize = dest.into();
                if used_registers[dest] {
                    result.push(instruction);
                }
            }
            IrInstruction::MvArg { dest, .. } => {
                let dest: usize = dest.into();
                if used_registers[dest] {
                    result.push(instruction);
                }
            }
            IrInstruction::BinOp {
                dest,
                op1,
                op2,
                operator: _,
            } => {
                let dest: usize = dest.into();
                if used_registers[dest] {
                    let op1: usize = op1.into();
                    let op2: usize = op2.into();
                    used_registers[op1] = true;
                    used_registers[op2] = true;
                    result.push(instruction);
                }
            }
            IrInstruction::Neg { dest, op } => {
                let dest: usize = dest.into();
                if used_registers[dest] {
                    let op: usize = op.into();
                    used_registers[op] = true;
                    result.push(instruction);
                }
            }
            IrInstruction::Call { dest, ref args, .. } => {
                let dest: usize = dest.into();
                if used_registers[dest] {
                    for arg in args {
                        let arg: usize = (*arg).into();
                        used_registers[arg] = true;
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

fn rename_registers(body: Vec<IrInstruction>, num_used_registers: usize) -> ReplacedBody {
    // By default, each register maps to itself
    let mut register_replacement: Vec<IrRegister> = Vec::with_capacity(num_used_registers);
    for i in 0..num_used_registers {
        register_replacement.push(IrRegister::from_u32(i as u32));
    }

    let mut next_expected_register = 0;
    let mut result = Vec::with_capacity(body.len());
    for instruction in body {
        match instruction {
            IrInstruction::Ret { reg } => {
                // Ret is the only instruction that does increment next_expect_register
                // because it does not alllocate a new register
                let reg: usize = reg.into();
                result.push(IrInstruction::Ret {
                    reg: register_replacement[reg],
                });
            }
            IrInstruction::Mvi { dest, val } => {
                let dest: usize = dest.into();
                if next_expected_register == dest {
                    result.push(instruction.clone());
                } else {
                    let replaced_register = IrRegister::from_u32(next_expected_register as u32);
                    result.push(IrInstruction::Mvi {
                        dest: replaced_register,
                        val,
                    });
                    register_replacement[dest] = replaced_register;
                }
                next_expected_register += 1;
            }
            IrInstruction::MvArg { dest, arg } => {
                let dest: usize = dest.into();
                if next_expected_register == dest {
                    result.push(instruction.clone());
                } else {
                    let replaced_register = IrRegister::from_u32(next_expected_register as u32);
                    result.push(IrInstruction::MvArg {
                        dest: replaced_register,
                        arg,
                    });
                    register_replacement[dest] = replaced_register;
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
                let dest_usize: usize = dest.into();
                let op1: usize = op1.into();
                let op2: usize = op2.into();
                if next_expected_register == dest_usize {
                    result.push(IrInstruction::BinOp {
                        operator,
                        dest,
                        op1: register_replacement[op1],
                        op2: register_replacement[op2],
                    });
                } else {
                    let replaced_register = IrRegister::from_u32(next_expected_register as u32);
                    result.push(IrInstruction::BinOp {
                        operator,
                        dest: replaced_register,
                        op1: register_replacement[op1],
                        op2: register_replacement[op2],
                    });
                    register_replacement[dest_usize] = replaced_register;
                }
                next_expected_register += 1;
            }
            IrInstruction::Neg { dest, op } => {
                let dest_usize: usize = dest.into();
                let op: usize = op.into();
                if next_expected_register == dest_usize {
                    result.push(IrInstruction::Neg {
                        dest,
                        op: register_replacement[op],
                    });
                } else {
                    let replaced_register = IrRegister::from_u32(next_expected_register as u32);
                    result.push(IrInstruction::Neg {
                        dest: replaced_register,
                        op: register_replacement[op],
                    });
                    register_replacement[dest_usize] = replaced_register;
                }
                next_expected_register += 1;
            }
            IrInstruction::Call {
                dest,
                name,
                function_id,
                args,
            } => {
                let dest_usize: usize = dest.into();

                let args = args
                    .into_iter()
                    .map(|arg| {
                        let arg: usize = arg.into();
                        register_replacement[arg]
                    })
                    .collect();

                if next_expected_register == dest_usize {
                    result.push(IrInstruction::Call {
                        dest,
                        name,
                        function_id,
                        args,
                    });
                } else {
                    let replaced_register = IrRegister::from_u32(next_expected_register as u32);
                    result.push(IrInstruction::Call {
                        dest: replaced_register,
                        name,
                        function_id,
                        args,
                    });
                    register_replacement[dest_usize] = replaced_register;
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
    use crate::{
        ir::builders::{add, call, mvi},
        optimization::rename_registers,
    };

    use super::deduplicate_constants;

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
            mvi(2, 1),
            add(3, 1, 0),
            call(4, "f", 0, vec![3]),
        ];
        let optimized = deduplicate_constants(body, 4);

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
