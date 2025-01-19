#[allow(unused)]
use rustix::mm::{mmap_anonymous, mprotect, MapFlags, MprotectFlags, ProtFlags};
use thiserror::Error;

#[allow(unused)]
use crate::backend_aarch64::Aarch64Generator;
#[allow(unused)]
use crate::backend_x64_linux::X64LinuxGenerator;

use crate::{
    backend::{BackendError, CompiledFunctionCatalog, FunctionId, MachineCodeGenerator},
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
    Backend(#[from] BackendError),
    #[error("{0}")]
    Jit(#[from] MmapError),
    #[error("main function {0} not found")]
    MainFunctionNotFound(String),
}

#[derive(Debug)]
pub struct JitProgram {
    pub function_catalog: Box<CompiledFunctionCatalog>,
    pub main_function: fn() -> i64,
}

pub fn jit_compile_program(source: &str, main_function_name: &str) -> Result<JitProgram, JitError> {
    println!("source:");
    println!("{}", source);
    println!();

    let program = parser::parse_program(source)?;
    let compiled_functions = frontend::compile(program)?;

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    let mut gen = X64LinuxGenerator::default();
    #[cfg(target_arch = "aarch64")]
    let mut gen = Aarch64Generator::default();

    let mut function_catalog = Box::new(CompiledFunctionCatalog::new(&compiled_functions));
    let function_catalog_ptr: *const CompiledFunctionCatalog = &*function_catalog;
    println!("function catalog: {:0X}", function_catalog_ptr as usize);

    let mut main_function = None;
    for function in compiled_functions.iter() {
        println!("compiling function: {}", function.name);
        println!("ir:");
        println!("{}", function);
        println!();

        let machine_code = gen.generate_machine_code(function, &function_catalog)?;
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

        let fun_ptr = unsafe { to_function_pointer(&machine_code.machine_code)? };
        function_catalog.store_function_pointer(
            function_catalog.get_function_id(function.name).unwrap(),
            fun_ptr,
        );

        if main_function_name == function.name {
            main_function = Some(fun_ptr);
        }
    }

    if let Some(main_function) = main_function {
        Ok(JitProgram {
            function_catalog,
            main_function,
        })
    } else {
        Err(JitError::MainFunctionNotFound(
            main_function_name.to_string(),
        ))
    }
}

pub fn jit_call_trampoline(
    function_catalog_ptr: *const CompiledFunctionCatalog,
    function_index: usize,
) -> i64 {
    println!(
        "in trampoline with args {:?} {}",
        function_catalog_ptr, function_index
    );
    let function_catalog = unsafe { &*function_catalog_ptr };
    let fun = function_catalog.get_function_pointer(FunctionId(function_index));
    println!("function pointer found: {:?}", fun);

    let result = fun();
    println!("function result: {}", result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_compile_basic_function() {
        let source = "fn test() { let a = 2; return a + 1; }";
        let program = super::jit_compile_program(source, "test").expect("function should compile");
        let res = (program.main_function)() as f64;
        assert_eq!(res, 3.0);
    }

    #[test]
    fn syntax_errors_are_handled() {
        let source = "fn invalid";
        let err = super::jit_compile_program(source, "foo").expect_err("should have not compiled");
        assert!(matches!(err, JitError::Parser(_)));
    }

    #[test]
    fn main_function_not_found_is_an_error() {
        let source = "fn f() { return 42; }";
        let err = super::jit_compile_program(source, "main")
            .expect_err("should not have found the main function");
        assert!(matches!(err, JitError::MainFunctionNotFound(_)));
    }
}
