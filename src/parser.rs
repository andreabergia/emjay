use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;
use thiserror::Error;

use crate::ast::{Block, BlockElement, Expression, Function, FunctionCall, Program};
use crate::grammar::{EmjayGrammar, Rule};

fn parse_expression(rule: Pair<'_, Rule>) -> Expression {
    let pratt = crate::grammar::pratt_parser();
    pratt
        .map_primary(|primary| match primary.as_rule() {
            Rule::number => Expression::Number(primary.as_str().parse().unwrap()),
            Rule::identifier => Expression::Identifier(primary.as_str()),
            Rule::expression => parse_expression(primary),
            Rule::functionCall => Expression::FunctionCall(parse_function_call(primary)),
            _ => unreachable!(""),
        })
        .map_prefix(|prefix, right| match prefix.as_rule() {
            Rule::neg => Expression::Negate(Box::new(right)),
            _ => unreachable!(),
        })
        .map_infix(|left, op, right| match op.as_rule() {
            Rule::add => Expression::Add(Box::new(left), Box::new(right)),
            Rule::sub => Expression::Sub(Box::new(left), Box::new(right)),
            Rule::mul => Expression::Mul(Box::new(left), Box::new(right)),
            Rule::div => Expression::Div(Box::new(left), Box::new(right)),
            Rule::pow => Expression::Pow(Box::new(left), Box::new(right)),
            Rule::rem => Expression::Rem(Box::new(left), Box::new(right)),
            _ => unreachable!(),
        })
        .parse(rule.into_inner())
}

fn parse_function_call(rule: Pair<'_, Rule>) -> FunctionCall {
    let mut inner = rule.into_inner();
    let name = inner.next().unwrap().as_str();
    FunctionCall { name }
}

fn parse_statement_let(rule: Pair<'_, Rule>) -> BlockElement {
    let mut inner = rule.into_inner();
    let name = inner.next().unwrap().as_str();
    let expression = parse_expression(inner.next().unwrap());
    BlockElement::LetStatement { name, expression }
}

fn parse_statement_assignment(rule: Pair<'_, Rule>) -> BlockElement {
    let mut inner = rule.into_inner();
    let name = inner.next().unwrap().as_str();
    let expression = parse_expression(inner.next().unwrap());
    BlockElement::AssignmentStatement { name, expression }
}

fn parse_statement_return(rule: Pair<'_, Rule>) -> BlockElement {
    let mut inner = rule.into_inner();
    let expression = parse_expression(inner.next().unwrap());
    BlockElement::ReturnStatement(expression)
}

fn parse_block(rule: Pair<'_, Rule>) -> Block {
    rule.into_inner()
        .map(|statement| match statement.as_rule() {
            Rule::letStatement => parse_statement_let(statement),
            Rule::assignmentStatement => parse_statement_assignment(statement),
            Rule::returnStatement => parse_statement_return(statement),
            Rule::block => BlockElement::NestedBlock(parse_block(statement)),
            _ => unreachable!(),
        })
        .collect()
}

fn parse_function(rule: Pair<'_, Rule>) -> Function {
    let mut rule = rule.into_inner();
    let name = rule.next().unwrap().as_str();
    let block = parse_block(rule.next().unwrap());
    Function { name, block }
}

#[derive(Debug, Error)]
#[error("parse error: {wrapped}")]
pub struct ParseError {
    #[from]
    wrapped: Error<Rule>,
}

pub fn parse_program(program: &str) -> Result<Program, Box<ParseError>> {
    let mut parsed = EmjayGrammar::parse(Rule::program, program).map_err(ParseError::from)?;
    let parsed = parsed.next().unwrap();

    let mut functions: Program = Default::default();
    for rule in parsed.into_inner() {
        match rule.as_rule() {
            Rule::functionDeclaration => functions.push(parse_function(rule)),
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    Ok(functions)
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{BlockElement, Expression, Function, FunctionCall},
        parser::parse_program,
    };

    #[test]
    fn can_parse_program() {
        let program = parse_program(
            r"fn foo() {
            let x = -y + 3 * (z() - 1);
            {
                let z = 42;
            }
            return x;
        }",
        )
        .expect("should have been able to parse program");
        assert_eq!(
            vec![Function {
                name: "foo",
                block: vec![
                    BlockElement::LetStatement {
                        name: "x",
                        expression: Expression::Add(
                            Box::new(Expression::Negate(Box::new(Expression::Identifier("y")))),
                            Box::new(Expression::Mul(
                                Box::new(Expression::Number(3f64)),
                                Box::new(Expression::Sub(
                                    Box::new(Expression::FunctionCall(FunctionCall { name: "z" })),
                                    Box::new(Expression::Number(1f64))
                                ))
                            ))
                        )
                    },
                    BlockElement::NestedBlock(vec![BlockElement::LetStatement {
                        name: "z",
                        expression: Expression::Number(42f64)
                    }]),
                    BlockElement::ReturnStatement(Expression::Identifier("x")),
                ]
            }],
            program
        );
    }

    #[test]
    fn syntax_errors_are_caught() {
        let program = parse_program(r"invalid");
        assert!(program.is_err());
    }
}
