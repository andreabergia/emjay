use std::collections::HashMap;

use thiserror::Error;

use crate::ir::CompiledFunction;

pub trait MachineCodeGenerator {
    fn generate_machine_code(
        &mut self,
        function: &CompiledFunction,
        function_catalog: &CompiledFunctionCatalog,
    ) -> Result<GeneratedMachineCode, BackendError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(pub usize);

pub struct GeneratedMachineCode {
    pub asm: String,
    pub machine_code: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("function not found: {0}")]
    FunctionNotFound(String),
}

pub type JitFn = fn(i64, i64, i64, i64, i64, i64) -> i64;

/// Stores two maps:
/// - function name -> a progressive ID
/// - progressive ID -> address (after it has been mmap-ed)
#[derive(Debug)]
pub struct CompiledFunctionCatalog {
    functions_by_name: HashMap<String, FunctionId>,

    // Indexed by FunctionId, which are dense
    addresses: Vec<JitFn>,
}

impl CompiledFunctionCatalog {
    pub fn new(program: &[CompiledFunction]) -> Self {
        let functions: HashMap<_, _> = program
            .iter()
            .enumerate()
            .map(|(index, function)| (function.name.to_string(), FunctionId(index)))
            .collect();
        Self {
            functions_by_name: functions,
            addresses: Vec::with_capacity(program.len()),
        }
    }

    pub fn get_function_id(&self, name: &str) -> Option<FunctionId> {
        self.functions_by_name.get(name).copied()
    }

    /// Stores a function pointer. Requirement: it must be called in order of `id`
    /// and;for each function in the program
    pub fn store_function_pointer(&mut self, id: FunctionId, fun_ptr: JitFn) {
        assert!(id.0 == self.addresses.len());
        self.addresses.insert(id.0, fun_ptr);
    }

    pub fn get_function_pointer(&self, id: FunctionId) -> JitFn {
        assert!(id.0 < self.addresses.len());
        self.addresses[id.0]
    }
}
