use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;

use crate::ast::{Block, BlockElement, Expression, Function, Program};
use crate::grammar::{EmjayGrammar, Rule};

fn parse_expression(rule: Pair<'_, Rule>) -> Expression {
    let pratt = crate::grammar::pratt_parser();
    pratt
        .map_primary(|primary| match primary.as_rule() {
            Rule::number => Expression::Number(primary.as_str().parse().unwrap()),
            Rule::identifier => Expression::Identifier(primary.as_str()),
            _ => unreachable!(),
        })
        .map_prefix(|prefix, right| match prefix.as_rule() {
            Rule::neg => Expression::Negate(Box::new(right)),
            _ => unreachable!(),
        })
        .map_postfix(|left, postfix| match postfix.as_rule() {
            Rule::fac => Expression::Fact(Box::new(left)),
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

fn parse_let(rule: Pair<'_, Rule>) -> BlockElement {
    let mut inner = rule.into_inner();
    let name = inner.next().unwrap().as_str();
    let expression = parse_expression(inner.next().unwrap());
    BlockElement::LetStatement { name, expression }
}

fn parse_assignment(rule: Pair<'_, Rule>) -> BlockElement {
    let mut inner = rule.into_inner();
    let name = inner.next().unwrap().as_str();
    let expression = parse_expression(inner.next().unwrap());
    BlockElement::AssignmentStatement { name, expression }
}

fn parse_block(rule: Pair<'_, Rule>) -> Block {
    rule.into_inner()
        .map(|statement| match statement.as_rule() {
            Rule::letStatement => parse_let(statement),
            Rule::assignmentStatement => parse_assignment(statement),
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

fn parse_program(program: &str) -> Result<Program, Error<Rule>> {
    let mut parsed = EmjayGrammar::parse(Rule::program, program)?;
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
    use crate::parser::parse_program;

    #[test]
    fn can_parse_program() {
        let p = parse_program("fn foo() { let x = -y + 3 * z!; { let z = 42; } }");
        assert!(p.is_ok());
        println!("{:?}", p);
    }
}
