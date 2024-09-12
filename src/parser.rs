use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct EmjayParser;

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
struct Expression {
    identifier: String,
}

fn parse_expression(rule: Pair<'_, Rule>) -> Result<Expression, Error<Rule>> {
    let expression = rule.as_str();
    return Ok(Expression {
        identifier: expression.to_string(),
    });
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
    let mut parsed = EmjayParser::parse(Rule::program, program)?;
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

    use super::{EmjayParser, Rule};
    use pest::Parser;

    #[test]
    fn grammar_can_parse_let() {
        let parsed = EmjayParser::parse(Rule::letStatement, "let x1 = y")
            .expect("can parse simple statement")
            .next()
            .unwrap();
        match parsed.as_rule() {
            Rule::letStatement => {
                let mut inner = parsed.into_inner();
                let id = inner.next().unwrap().as_str();
                let expression = inner.next().unwrap().as_str();
                println!("id: {}, expr: {}", id, expression);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn grammar_can_parse_program() {
        let p = EmjayParser::parse(
            Rule::program,
            r"
        fn foo() {
            let x1 = y;
        }

        fn bar() {
            let x1 = x;
            let x2 = y;
            x3 = z;
        }

        fn empty() {}
        ",
        )
        .expect("can parse simple program")
        .next()
        .unwrap();
        println!("{:?}", p);
    }

    #[test]
    fn can_parse_program() {
        let p = parse_program("fn foo() { let x = y; }");
        println!("{:?}", p);
    }
}
