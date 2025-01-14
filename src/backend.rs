use std::collections::HashMap;

use thiserror::Error;

use crate::ir::CompiledFunction;

pub trait MachineCodeGenerator {
    fn generate_machine_code<FC>(
        &mut self,
        function: &CompiledFunction,
        function_catalog: &FC,
    ) -> Result<GeneratedMachineCode, BackendError>
    where
        FC: FunctionCatalog;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(usize);

pub trait FunctionCatalog {
    fn get_function_id(&self, name: &str) -> Option<FunctionId>;
}

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

pub struct CompiledFunctionCatalog {
    functions: HashMap<String, FunctionId>,
}

impl CompiledFunctionCatalog {
    pub fn new(program: &[CompiledFunction]) -> Self {
        let functions: HashMap<_, _> = program
            .iter()
            .enumerate()
            .map(|(index, function)| (function.name.to_string(), FunctionId(index)))
            .collect();
        Self { functions }
    }
}

impl FunctionCatalog for CompiledFunctionCatalog {
    fn get_function_id(&self, name: &str) -> Option<FunctionId> {
        self.functions.get(name).copied()
    }
}
