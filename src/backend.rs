use crate::ir::CompiledFunction;

pub trait MachineCodeGenerator {
    fn generate_machine_code(&mut self, function: &CompiledFunction) -> GeneratedMachineCode;
}

pub struct GeneratedMachineCode {
    pub asm: String,
    pub machine_code: Vec<u8>,
}
