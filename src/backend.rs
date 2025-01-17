use std::collections::HashMap;

use thiserror::Error;

use crate::ir::CompiledFunction;

pub trait MachineCodeGenerator {
    fn generate_machine_code(
        &mut self,
        function: &CompiledFunction,
        function_catalog: &Box<CompiledFunctionCatalog>,
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

#[derive(Debug)]
pub struct CompiledFunctionCatalog {
    functions: HashMap<String, FunctionId>,

    // TODO: could be a vec, no need for a hashmap
    addresses: HashMap<FunctionId, fn() -> i64>,
}

impl CompiledFunctionCatalog {
    pub fn new(program: &[CompiledFunction]) -> Self {
        let functions: HashMap<_, _> = program
            .iter()
            .enumerate()
            .map(|(index, function)| (function.name.to_string(), FunctionId(index)))
            .collect();
        Self {
            functions,
            addresses: HashMap::new(),
        }
    }

    pub fn get_function_id(&self, name: &str) -> Option<FunctionId> {
        self.functions.get(name).copied()
    }

    pub fn store_function_pointer(&mut self, id: FunctionId, fun_ptr: fn() -> i64) {
        self.addresses.insert(id, fun_ptr);
    }

    pub fn get_function_pointer(&self, id: FunctionId) -> Option<fn() -> i64> {
        self.addresses.get(&id).copied()
    }
}
