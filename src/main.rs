#![allow(dead_code)]

use parser::parse_program;

mod ast;
mod backend;
mod frontend;
mod grammar;
mod ir;
mod parser;

fn main() {
    println!("Hello, world!");
    parse_program("fn foo() {}").unwrap();
}
