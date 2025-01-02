use rustix::mm::{mmap_anonymous, mprotect, MapFlags, MprotectFlags, ProtFlags};
use thiserror::Error;

#[cfg(target_arch = "aarch64")]
use crate::backend_aarch64::Aarch64Generator;
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
use crate::backend_x64_linux::X64LinuxGenerator;

use crate::{backend::MachineCodeGenerator, frontend, parser};

pub fn to_function_pointer(bytes: &[u8]) -> fn() -> i64 {
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    unsafe {
        let size = bytes.len();
        let map = mmap_anonymous(
            std::ptr::null_mut(),
            size,
            ProtFlags::WRITE | ProtFlags::EXEC,
            MapFlags::PRIVATE,
        )
        .unwrap();

        println!("mmapped address: {:?}", map);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), map as *mut u8, size);

        let f: fn() -> i64 = std::mem::transmute(map);
        f
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    unsafe {
        let size = bytes.len();
        let map = mmap_anonymous(
            std::ptr::null_mut(),
            size,
            ProtFlags::WRITE,
            MapFlags::PRIVATE,
        )
        .unwrap();

        println!("mmapped address: {:?}", map);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), map as *mut u8, size);

        mprotect(map, size, MprotectFlags::EXEC).unwrap();
        println!("mprotected: {:?}", map);

        let f: fn() -> i64 = std::mem::transmute(map);
        f
    }
}

#[derive(Debug, Error)]
pub enum JitError {
    #[error("{0}")]
    ParseError(#[from] Box<parser::ParseError>),
}

pub fn jit_compile_fn(source: &str) -> Result<fn() -> i64, JitError> {
    println!("source:");
    println!("{}", source);
    println!();

    let program = parser::parse_program(source)?;
    let compiled = frontend::compile(program);
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

    Ok(to_function_pointer(&machine_code.machine_code))
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
        assert!(matches!(err, JitError::ParseError(_)));
    }
}
