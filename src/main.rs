#![allow(dead_code)]

use backend::MachineCodeGenerator;
use backend_x64_linux::X64LinuxGenerator;
use parser::parse_program;
use rustix::mm::{mmap_anonymous, mprotect, MapFlags, MprotectFlags, ProtFlags};

mod ast;
mod backend;
mod backend_x64_linux;
mod frontend;
mod grammar;
mod ir;
mod parser;

fn call_fn(bytes: &[u8]) -> f64 {
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
        f() as f64
    }
}

fn main() {
    let source = "fn the_answer() { let a = 12; let b = 42; return a + b; }";
    println!("source:");
    println!("{}", source);
    println!();

    let program = parse_program(source).unwrap();
    let compiled = frontend::compile(program);
    assert_eq!(compiled.len(), 1);
    println!("ir:");
    println!("{}", compiled[0]);
    println!();

    let mut gen = X64LinuxGenerator::default();
    let machine_code = gen.generate_machine_code(&compiled[0]);
    println!("asm:");
    println!("{}", machine_code.asm);

    println!("Machine code:");
    machine_code
        .machine_code
        .iter()
        .for_each(|byte| print!("{:02X} ", byte));
    println!();
    println!();

    let res = call_fn(&machine_code.machine_code);
    println!("function result: {}", res);
}
