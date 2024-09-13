use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;

use crate::grammar::{EmjayGrammar, Rule};

#[derive(Debug)]
struct Function {
    name: String,
    statements: Block,
}

type Program = Vec<Function>;

#[derive(Debug)]
enum Statement {
    LetStatement {
        name: String,
        expression: Expression,
    },
    AssignmentStatement {
        name: String,
        expression: Expression,
    },
}

type Block = Vec<Statement>;

#[derive(Debug)]
enum Expression {
    Identifier(String),
    Number(f64),
    Negate(Box<Expression>),
    Add(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),
    Mul(Box<Expression>, Box<Expression>),
    Div(Box<Expression>, Box<Expression>),
    Pow(Box<Expression>, Box<Expression>),
    Rem(Box<Expression>, Box<Expression>),
    Fact(Box<Expression>),
}

fn parse_expression(rule: Pair<'_, Rule>) -> Result<Expression, Error<Rule>> {
    let pratt = crate::grammar::pratt_parser();
    Ok(pratt
        .map_primary(|primary| match primary.as_rule() {
            Rule::number => Expression::Number(primary.as_str().parse().unwrap()),
            Rule::identifier => Expression::Identifier(primary.as_str().to_owned()),
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
        .parse(rule.into_inner()))
}

fn parse_let(rule: Pair<'_, Rule>) -> Result<Statement, Error<Rule>> {
    let mut inner = rule.into_inner();
    let id = inner.next().unwrap().as_str();
    let expression = parse_expression(inner.next().unwrap())?;
    Ok(Statement::LetStatement {
        name: id.to_string(),
        expression,
    })
}

fn parse_assignment(rule: Pair<'_, Rule>) -> Result<Statement, Error<Rule>> {
    let mut inner = rule.into_inner();
    let id = inner.next().unwrap().as_str();
    let expression = parse_expression(inner.next().unwrap())?;
    Ok(Statement::AssignmentStatement {
        name: id.to_string(),
        expression,
    })
}

fn parse_block(rule: Pair<'_, Rule>) -> Result<Block, Error<Rule>> {
    let block: Result<Block, Error<Rule>> = rule
        .into_inner()
        .map(|statement| match statement.as_rule() {
            Rule::letStatement => parse_let(statement),
            Rule::assignmentStatement => parse_assignment(statement),
            _ => unreachable!(),
        })
        .collect();
    block
}

fn parse_function(rule: Pair<'_, Rule>) -> Result<Function, Error<Rule>> {
    let mut rule = rule.into_inner();
    let id = rule.next().unwrap().as_str();
    let block = parse_block(rule.next().unwrap())?;
    Ok(Function {
        name: id.to_string(),
        statements: block,
    })
}

fn parse_program(program: &str) -> Result<Program, Error<Rule>> {
    let mut parsed = EmjayGrammar::parse(Rule::program, program)?;
    let parsed = parsed.next().unwrap();

    let mut functions: Program = Default::default();
    for rule in parsed.into_inner() {
        match rule.as_rule() {
            Rule::functionDeclaration => functions.push(parse_function(rule)?),
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
        let p = parse_program("fn foo() { let x = -y + 3 * z!; }");
        assert!(p.is_ok());
        println!("{:?}", p);
    }
}
