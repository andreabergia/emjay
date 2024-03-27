use crate::lexer::Token;
use logos::{Lexer, Logos};

#[derive(Default, Debug)]
struct ProgramNode {
    statements: Vec<StatementNode>,
}

#[derive(Debug)]
enum StatementNode {
    Let {
        identifier: String,
        expression: ExpressionNode,
    },
    Assignment {
        identifier: String,
        expression: ExpressionNode,
    },
}

#[derive(Debug)]
struct ExpressionNode {
    term: TermNode,
    rest: Option<Box<ExpressionNodeRest>>,
}

#[derive(Debug)]
struct ExpressionNodeRest {
    operator: ExpressionNodeRestOperator,
    expression: ExpressionNode,
}

#[derive(Debug)]
enum ExpressionNodeRestOperator {
    Plus,
    Minus,
}

#[derive(Debug)]
struct TermNode {
    factor: FactorNode,
    rest: Option<Box<TermNodeRest>>,
}

#[derive(Debug)]
struct TermNodeRest {
    operator: TermNodeRestOperator,
    term: TermNode,
}

#[derive(Debug)]
enum TermNodeRestOperator {
    Multiply,
    Divide,
}

#[derive(Debug)]
enum FactorNode {
    Identifier(String),
    NumericLiteral(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsingError {
    SomeError(String),
}

fn parse_statement(lexer: &mut Lexer<'_, Token>) -> Result<Option<StatementNode>, ParsingError> {

    // if let Some(token) = lexer.next() {
    //      match token {
    //          Ok(Token::Identifier(id)) => Ok(StatementNode::Assignment {
    //              identifier: (),
    //              expression: (),
    //          }),

    //          _ => {
    //              return Err(ParsingError::SomeError(format!(
    //                  "unexpected token {:?} at position {:?}",
    //                  lexer.span(),
    //                  lexer.span()
    //              )))
    //          }
    //      }
    //  }
}

pub fn parse(source: &str) -> Result<ProgramNode, ParsingError> {
    let mut lexer = Token::lexer(source);
    let mut program = ProgramNode::default();

    loop {
        while let Some(statement) = parse_statement(&mut lexer)? {
            program.statements.push(statement)
        }
    }

    Ok(program)
}
