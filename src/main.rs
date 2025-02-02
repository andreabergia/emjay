#![allow(dead_code)]

use jit::jit_compile_program;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

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
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default logging subscriber failed");

    let source = r"
        fn main() {
            return 1000 + f(3, 2, 1);
        }

        fn f(x, y, z) {
            return x * 100 + y * 10 + (g(z) + z) * 2;
        }

        fn g(z) {
            return z + 1;
        }
    ";

    let jit_program = jit_compile_program(source, "main").expect("program should compile");
    info!("program compiled, running it!");
    let fun = jit_program.main_function;
    info!("main function result: {}", fun(0, 0, 0, 0, 0, 0));
}
