use parser::parse_program;

mod ast;
mod grammar;
mod parser;

fn main() {
    println!("Hello, world!");
    parse_program("fn foo() {}").unwrap();
}
