#![allow(dead_code)]

use backend::MachineCodeGenerator;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
use backend_aarch64_macos::Aarch64MacOsGenerator;
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
use backend_x64_linux::X64LinuxGenerator;
use parser::parse_program;
use rustix::mm::{mmap_anonymous, mprotect, MapFlags, MprotectFlags, ProtFlags};

mod ast;
mod backend;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
mod backend_aarch64_macos;
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod backend_x64_linux;
mod frontend;
mod grammar;
mod ir;
mod parser;

fn call_fn(bytes: &[u8]) -> f64 {
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
        f() as f64
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
        f() as f64
    }
}

fn main() {
    // let source = "fn the_answer() { let a = 12; let b = 42; return a + b; }";
    let source = "fn the_answer() { let a = 1; return a; }";
    println!("source:");
    println!("{}", source);
    println!();

    let program = parse_program(source).unwrap();
    let compiled = frontend::compile(program);
    assert_eq!(compiled.len(), 1);
    println!("ir:");
    println!("{}", compiled[0]);
    println!();

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    let mut gen = X64LinuxGenerator::default();
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    let mut gen = Aarch64MacOsGenerator::default();

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
