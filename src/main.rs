#![allow(dead_code)]

use jit::jit_compile_program;

mod ast;
mod backend;
mod backend_aarch64;
mod backend_register_allocator;
mod backend_x64_linux;
mod frontend;
mod grammar;
mod ir;
mod jit;
mod parser;
mod program_counter;

fn main() {
    let source = r"
        fn main() {
            return f() + 1;
        }

        fn f() {
            return 42;
        }
    ";
    let jit_program = jit_compile_program(source, "main").expect("program should compile");
    let res = (jit_program.main_function)() as f64;
    println!("main function result: {}", res);
}
