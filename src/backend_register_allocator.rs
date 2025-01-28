use core::fmt;
use std::collections::VecDeque;

use crate::{
    ir::{CompiledFunction, IrRegister},
    program_counter::ProgramCounter,
};

#[derive(Debug, Clone, PartialEq)]
pub enum AllocatedLocation<HardwareRegister> {
    Register { register: HardwareRegister },
    Stack { offset: usize },
}

/// Computes `ir_reg_used_at`, mapping each ir_reg to the PCs where it is used
/// Key: ir_reg, value: PCs where the register is used
fn compute_ir_reg_used_at(function: &CompiledFunction) -> Vec<VecDeque<ProgramCounter>> {
    let mut ir_reg_used_at = vec![VecDeque::new(); function.num_used_registers];
    for (pc, instruction) in function.body.iter().enumerate() {
        let pc = ProgramCounter(pc);
        for ir_reg in instruction.operands() {
            ir_reg_used_at[usize::from(ir_reg)].push_back(pc);
        }
    }

    // Debug
    println!("  computed usage:");
    for (ir_reg, used_at) in ir_reg_used_at.iter().enumerate() {
        println!("    reg {} used at {:?}", ir_reg, used_at);
    }
    println!();

    ir_reg_used_at
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LogicalHwRegister(usize);

impl fmt::Display for LogicalHwRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

const NOT_ALLOCATED: LogicalHwRegister = LogicalHwRegister(usize::MAX);

/// Allocates all ir registers to a logical hw register, reusing the hw registers
/// when possible. Result key: ir_reg, value: logical_hw_reg
fn allocate_ir_regs_to_logical_hw_regs(
    function: &CompiledFunction,
    mut ir_reg_used_at: Vec<VecDeque<ProgramCounter>>,
) -> Vec<LogicalHwRegister> {
    // Key: ir_reg, value: logical_hw_reg
    let mut ir_reg_allocation = vec![NOT_ALLOCATED; function.num_used_registers];
    // Key: logical_hw_reg, value: ir_reg
    let mut logical_hw_regs_content: Vec<IrRegister> = Vec::new();

    const FREE: IrRegister = IrRegister::from_u32(u32::MAX);
    let mut free_logical_hw_registers: Vec<LogicalHwRegister> = Vec::new();

    for (pc, instruction) in function.body.iter().enumerate() {
        let pc = ProgramCounter(pc);
        println!("  pc {:2}:  {}", pc.0, instruction);
        for ir_reg in instruction.operands() {
            print!("    register {} ", ir_reg);

            if ir_reg_allocation[usize::from(ir_reg)] != NOT_ALLOCATED {
                // Already allocated
                println!(
                    "already allocated to hw reg {}",
                    ir_reg_allocation[usize::from(ir_reg)]
                );
            } else if free_logical_hw_registers.is_empty() {
                // Requires a new logical hw register
                let new_logical_hw_reg = LogicalHwRegister(logical_hw_regs_content.len());
                println!("allocating to new hw reg {:?}", new_logical_hw_reg);
                ir_reg_allocation[usize::from(ir_reg)] = new_logical_hw_reg;
                logical_hw_regs_content.push(ir_reg);
            } else {
                // We can reuse something free
                let first_free_reg = free_logical_hw_registers.pop().unwrap();
                println!("allocating to existing but free hw reg {}", first_free_reg);
                ir_reg_allocation[usize::from(ir_reg)] = first_free_reg;
                logical_hw_regs_content[first_free_reg.0] = ir_reg;
            }
        }

        // Can we free something?
        for (hw_reg, ir_reg) in logical_hw_regs_content.iter_mut().enumerate() {
            if *ir_reg != FREE {
                let ir_reg_used_at_pcs = &mut ir_reg_used_at[usize::from(*ir_reg)];
                if !ir_reg_used_at_pcs.is_empty() && ir_reg_used_at_pcs[0] == pc {
                    ir_reg_used_at_pcs.pop_front();
                }

                if ir_reg_used_at_pcs.is_empty() {
                    println!(
                        "    freeing register {:?} which was assigned to {} because it was its last usage",
                            hw_reg, *ir_reg
                        );
                    *ir_reg = FREE;
                    free_logical_hw_registers.push(LogicalHwRegister(hw_reg));
                }
            }
        }

        println!(
            "    ir_reg_allocation: [{}]",
            ir_reg_allocation
                .iter()
                .map(|r| if *r == NOT_ALLOCATED {
                    String::from("x")
                } else {
                    format!("{}", r)
                })
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "    logical_hw_regs: [{}]",
            logical_hw_regs_content
                .iter()
                .map(|h| if *h == FREE {
                    String::from("f")
                } else {
                    format!("{}", h)
                })
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    ir_reg_allocation
}

fn map_to_hw_register<HardwareRegister>(
    ir_reg_allocation: Vec<LogicalHwRegister>,
    hw_registers: Vec<HardwareRegister>,
) -> Vec<AllocatedLocation<HardwareRegister>>
where
    HardwareRegister: Clone,
{
    let num_hw_regs = hw_registers.len();
    ir_reg_allocation
        .iter()
        .map(|logical_hw_reg| {
            assert!(*logical_hw_reg != NOT_ALLOCATED);

            if logical_hw_reg.0 < num_hw_regs {
                AllocatedLocation::Register {
                    register: hw_registers[logical_hw_reg.0].clone(),
                }
            } else {
                AllocatedLocation::Stack {
                    offset: (logical_hw_reg.0 - num_hw_regs) * 8,
                }
            }
        })
        .collect()
}

pub fn allocate<HardwareRegister>(
    function: &CompiledFunction,
    hw_registers: Vec<HardwareRegister>,
) -> Vec<AllocatedLocation<HardwareRegister>>
where
    HardwareRegister: Clone,
{
    println!("allocating registers!");
    let ir_reg_used_at = compute_ir_reg_used_at(function);
    let ir_reg_allocation = allocate_ir_regs_to_logical_hw_regs(function, ir_reg_used_at);
    map_to_hw_register(ir_reg_allocation, hw_registers)
}

#[cfg(test)]
mod tests {
    use crate::{
        backend_register_allocator::{allocate, AllocatedLocation},
        ir::{
            builders::{add, mvi},
            CompiledFunction,
        },
    };

    #[test]
    fn can_allocate_and_handle_spillover() {
        let allocations = allocate(
            &CompiledFunction {
                name: "test",
                num_args: 0,
                body: vec![mvi(0, 0.0), mvi(1, 1.0), add(2, 0, 1)],
                num_used_registers: 3,
            },
            vec!["h0"],
        );

        assert_eq!(
            allocations,
            vec![
                AllocatedLocation::Register { register: "h0" },
                AllocatedLocation::Stack { offset: 0 },
                AllocatedLocation::Stack { offset: 8 },
            ]
        )
    }

    #[test]
    fn can_reuse_free_registers() {
        let allocations = allocate(
            &CompiledFunction {
                name: "test",
                num_args: 0,
                // Register h2 is unused after instruction #2, so we can reuse it for #3
                body: vec![mvi(0, 0.0), mvi(1, 1.0), mvi(2, 2.0), add(3, 0, 1)],
                num_used_registers: 4,
            },
            vec!["h0", "h1", "h2"],
        );

        assert_eq!(
            allocations,
            vec![
                AllocatedLocation::Register { register: "h0" },
                AllocatedLocation::Register { register: "h1" },
                AllocatedLocation::Register { register: "h2" },
                AllocatedLocation::Register { register: "h2" },
            ]
        )
    }
}
