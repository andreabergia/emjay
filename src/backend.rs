use thiserror::Error;

use crate::{frontend::FunctionId, ir::CompiledFunction};

pub trait MachineCodeGenerator {
    fn generate_machine_code(
        &mut self,
        function: &CompiledFunction,
        function_catalog: &CompiledFunctionCatalog,
    ) -> Result<GeneratedMachineCode, BackendError>;
}

pub struct GeneratedMachineCode {
    pub asm: String,
    pub machine_code: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("not implemented: {0}")]
    NotImplemented(String),
}

pub type JitFn = fn(i64, i64, i64, i64, i64, i64) -> i64;

#[derive(Debug)]
pub struct CompiledFunctionCatalog {
    // Indexed by FunctionId, which are dense. Thus, we can use a simple Vec
    // and avoid the extra cost of an hash map
    addresses: Vec<JitFn>,
}

impl CompiledFunctionCatalog {
    pub fn new(program: &[CompiledFunction]) -> Self {
        Self {
            addresses: Vec::with_capacity(program.len()),
        }
    }

    /// Stores a function pointer. Requirement: it must be called in order of `id`
    /// and for each function in the program
    pub fn store_function_pointer(&mut self, id: FunctionId, fun_ptr: JitFn) {
        assert!(id.0 == self.addresses.len());
        self.addresses.insert(id.0, fun_ptr);
    }

    pub fn get_function_pointer(&self, id: FunctionId) -> JitFn {
        assert!(id.0 < self.addresses.len());
        self.addresses[id.0]
    }
}
