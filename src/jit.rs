#[allow(unused)]
use rustix::mm::{mmap_anonymous, mprotect, MapFlags, MprotectFlags, ProtFlags};
use thiserror::Error;

#[allow(unused)]
use crate::backend_aarch64::Aarch64Generator;
#[allow(unused)]
use crate::backend_x64_linux::X64LinuxGenerator;

use crate::{
    backend::MachineCodeGenerator,
    frontend::{self, FrontendError},
    parser,
};

#[derive(Debug, Error)]
#[error("{description} (errno: {errno})")]
pub struct MmapError {
    description: String,
    errno: i32,
}

impl From<rustix::io::Errno> for MmapError {
    fn from(value: rustix::io::Errno) -> Self {
        Self {
            description: format!("mmap failed with error: {}", value),
            errno: value.raw_os_error(),
        }
    }
}

unsafe fn to_function_pointer(bytes: &[u8]) -> Result<fn() -> i64, MmapError> {
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    {
        let size = bytes.len();
        let map = mmap_anonymous(
            std::ptr::null_mut(),
            size,
            ProtFlags::WRITE | ProtFlags::EXEC,
            MapFlags::PRIVATE,
        )?;

        println!("mmapped address: {:?}", map);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), map as *mut u8, size);

        let f: fn() -> i64 = std::mem::transmute(map);
        Ok(f)
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        let size = bytes.len();
        let map = mmap_anonymous(
            std::ptr::null_mut(),
            size,
            ProtFlags::WRITE,
            MapFlags::PRIVATE,
        )?;

        println!("mmapped address: {:?}", map);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), map as *mut u8, size);

        mprotect(map, size, MprotectFlags::EXEC)?;
        println!("mprotected: {:?}", map);

        let f: fn() -> i64 = std::mem::transmute(map);
        Ok(f)
    }
}

#[derive(Debug, Error)]
pub enum JitError {
    #[error("{0}")]
    Parser(#[from] Box<parser::ParseError>),
    #[error("{0}")]
    Frontend(#[from] FrontendError),
    #[error("{0}")]
    Ji(#[from] MmapError),
}

pub fn jit_compile_fn(source: &str) -> Result<fn() -> i64, JitError> {
    println!("source:");
    println!("{}", source);
    println!();

    let program = parser::parse_program(source)?;
    let compiled = frontend::compile(program)?;
    assert_eq!(compiled.len(), 1);
    println!("ir:");
    println!("{}", compiled[0]);
    println!();

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    let mut gen = X64LinuxGenerator::default();
    #[cfg(target_arch = "aarch64")]
    let mut gen = Aarch64Generator::default();

    let machine_code = gen.generate_machine_code(&compiled[0]);
    println!("asm:");
    println!("{}", machine_code.asm);

    println!("Machine code:");
    for (index, byte) in machine_code.machine_code.iter().enumerate() {
        print!("{:02X} ", byte);
        if index % 4 == 3 {
            println!();
        }
    }
    println!();
    println!();

    let f = unsafe { to_function_pointer(&machine_code.machine_code)? };
    Ok(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_compile_basic_function() {
        let source = "fn test() { let a = 2; return a + 1; }";
        let fun = super::jit_compile_fn(source).expect("function should compile");
        let res = fun() as f64;
        assert_eq!(res, 3.0);
    }

    #[test]
    fn syntax_errors_are_handled() {
        let source = "fn invalid";
        let err = super::jit_compile_fn(source).expect_err("should have not compiled");
        assert!(matches!(err, JitError::Parser(_)));
    }
}
