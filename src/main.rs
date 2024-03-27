use winnow::{ascii::float, combinator::alt, prelude::*, token::literal};
mod lexer;
mod parser;

#[derive(Debug, PartialEq)]
enum AstNode {
    Scalar(f64),
    BinOp {
        op: Operator,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    Variable(String),
}

#[derive(Debug, PartialEq)]
enum Operator {
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
}

fn parse_scalar(s: &mut &str) -> PResult<AstNode> {
    float(s).map(|v| AstNode::Scalar(v))
}

fn parse_operator(s: &mut &str) -> PResult<Operator> {
    alt((
        literal("+").map(|_| Operator::Plus),
        literal("-").map(|_| Operator::Minus),
        literal("*").map(|_| Operator::Star),
        literal("/").map(|_| Operator::Slash),
        literal("^").map(|_| Operator::Caret),
        literal("%").map(|_| Operator::Percent),
    ))
    .parse_next(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scalar() {
        assert_eq!(parse_scalar(&mut "3.14"), Ok(AstNode::Scalar(3.14)));
    }

    #[test]
    fn test_parse_operator() {
        assert_eq!(parse_operator(&mut "+"), Ok(Operator::Plus));
        assert_eq!(parse_operator(&mut "-"), Ok(Operator::Minus));
        assert_eq!(parse_operator(&mut "*"), Ok(Operator::Star));
        assert_eq!(parse_operator(&mut "/"), Ok(Operator::Slash));
        assert_eq!(parse_operator(&mut "^"), Ok(Operator::Caret));
        assert_eq!(parse_operator(&mut "%"), Ok(Operator::Percent));
        assert!(parse_operator(&mut "x").is_err());
    }
}

fn main() {
    println!("Hello, world!");
}
