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
    let source =
        "fn main() { let a = 11; let b = 1; let c = a + 1; let d = b + 2; return c / (d - 1); }";
    let jit_program = jit_compile_program(source, "main").expect("program should compile");
    let res = (jit_program.main_function)() as f64;
    println!("function result: {}", res);
}
