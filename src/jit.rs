#[allow(unused)]
use rustix::mm::{mmap_anonymous, mprotect, MapFlags, MprotectFlags, ProtFlags};
use thiserror::Error;
use tracing::{debug, info};

#[allow(unused)]
use crate::backend_aarch64::Aarch64Generator;
#[allow(unused)]
use crate::backend_x64_linux::X64LinuxGenerator;

use crate::{
    backend::{BackendError, CompiledFunctionCatalog, JitFn, MachineCodeGenerator},
    frontend::{self, FrontendError, FunctionId},
    optimization, parser,
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

// Converts the given slice bytes, containing machine code, into a function pointer. It does so by
// mmapping a new page, copying the bytes, and then performing a cast.
unsafe fn to_function_pointer(bytes: &[u8]) -> Result<JitFn, MmapError> {
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    {
        let size = bytes.len();
        let map = mmap_anonymous(
            std::ptr::null_mut(),
            size,
            ProtFlags::WRITE | ProtFlags::EXEC,
            MapFlags::PRIVATE,
        )?;

        debug!("mmapped address: {:?}", map);
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

        debug!("mmapped address: {:?}", map);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), map as *mut u8, size);

        mprotect(map, size, MprotectFlags::EXEC)?;
        debug!("mprotected: {:?}", map);

        let f: JitFn = std::mem::transmute(map);
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
    pub main_function: JitFn,
}

pub fn jit_compile_program(source: &str, main_function_name: &str) -> Result<JitProgram, JitError> {
    info!("source: \n{}", source);

    let program = parser::parse_program(source)?;
    let compiled_functions = frontend::compile(program)?;

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    let mut gen = X64LinuxGenerator::default();
    #[cfg(target_arch = "aarch64")]
    let mut gen = Aarch64Generator::default();

    // Create the function catalog and stores it in a box, to ensure that it will be at a fixed
    // address and not be de-allocated
    let mut function_catalog = Box::new(CompiledFunctionCatalog::new(&compiled_functions));
    let function_catalog_ptr: *const CompiledFunctionCatalog = &*function_catalog;
    debug!("function catalog: {:0X}", function_catalog_ptr as usize);

    let mut main_function = None;
    for function in compiled_functions.iter() {
        debug!("compiling function: {}", function.name);
        debug!("base ir:\n{}", function);

        let function = &optimization::optimize_fun(function);
        debug!("optimized ir:\n{}", function);

        let machine_code = gen.generate_machine_code(function, &function_catalog)?;
        debug!("asm:\n{}", machine_code.asm);

        let machine_code_for_debug: String = machine_code
            .machine_code
            .iter()
            .enumerate()
            .map(|(index, byte)| {
                if index % 4 == 3 {
                    format!("{:02X}\n", byte)
                } else {
                    format!("{:02X} ", byte)
                }
            })
            .collect();
        debug!("Machine code:\n{}", machine_code_for_debug);

        let fun_ptr = unsafe { to_function_pointer(&machine_code.machine_code)? };
        function_catalog.store_function_pointer(function.id, fun_ptr);

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

/// This function acts as a trampoline to perform functions call from the a jit-ted function.
/// Since we first compile the function and then mmap-it, we do not have the address of
/// the called function when we're compiling the callee. Therefore, we use this trampoline.
/// When we're compiling the caller, we will replace the function to the callee with a call
/// to this trampoline function, passing the id of the callee. The trampoline will resolve
/// the actual address to which the callee has been mapped and will then invoke it.
/// As usual, most problems in computer science can be solved with an additional level of
/// indirection :-)
#[allow(clippy::too_many_arguments)]
pub fn jit_call_trampoline(
    function_catalog_ptr: *const CompiledFunctionCatalog,
    function_index: usize,
    a0: i64,
    a1: i64,
    a2: i64,
    a3: i64,
    a4: i64,
    a5: i64,
) -> i64 {
    debug!(
        "inside trampoline, with args {:?} {} - {} {} {} {} {} {}",
        function_catalog_ptr, function_index, a0, a1, a2, a3, a4, a5,
    );
    let function_catalog = unsafe { &*function_catalog_ptr };
    let fun = function_catalog.get_function_pointer(FunctionId(function_index));
    debug!("  function pointer found: {:?}", fun);

    let result = fun(a0, a1, a2, a3, a4, a5);

    debug!("  callee function result: {}", result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_generate_valid_basic_function() {
        let source = "fn test() { let a = 2; return -a + 1; }";
        let program = super::jit_compile_program(source, "test").expect("function should compile");
        let res = (program.main_function)(0, 0, 0, 0, 0, 0); // Call it!
        assert_eq!(res, -1);
    }

    #[test]
    fn can_generate_function_calls() {
        let source = "
        fn f(x) { return g() + x; }
        fn g() { return 1; }
        ";
        let program = super::jit_compile_program(source, "f").expect("function should compile");
        let res = (program.main_function)(4, 0, 0, 0, 0, 0); // Call it!
        assert_eq!(res, 5);
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
