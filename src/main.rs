#![allow(dead_code)]

use jit::jit_compile_fn;

mod ast;
mod backend;
#[cfg(target_arch = "aarch64")]
mod backend_aarch64;
mod backend_register_allocator;
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod backend_x64_linux;
mod frontend;
mod grammar;
mod ir;
mod jit;
mod parser;
mod program_counter;

fn main() {
    let source =
        "fn the_answer() { let a = 11; let b = 1; let c = a + 1; let d = b + 2; return c / (d - 1); }";
    let fun = jit_compile_fn(source).expect("function should compile");
    let res = fun() as f64;
    println!("function result: {}", res);
}
