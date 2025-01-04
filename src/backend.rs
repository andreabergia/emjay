use thiserror::Error;

use crate::ir::CompiledFunction;

pub trait MachineCodeGenerator {
    fn generate_machine_code(
        &mut self,
        function: &CompiledFunction,
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
