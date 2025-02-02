use std::sync::LazyLock;

use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
#[allow(dead_code)]
pub struct EmjayGrammar;

static EMJAY_PRATT_PARSER: LazyLock<PrattParser<Rule>> = LazyLock::new(|| {
    PrattParser::new()
        .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left))
        .op(Op::infix(Rule::mul, Assoc::Left) | Op::infix(Rule::div, Assoc::Left))
        .op(Op::infix(Rule::pow, Assoc::Right))
        .op(Op::prefix(Rule::neg))
});

pub fn pratt_parser() -> &'static PrattParser<Rule> {
    &EMJAY_PRATT_PARSER
}

#[cfg(test)]
mod tests {
    use super::{EmjayGrammar, Rule};
    use pest::Parser;

    fn assert_can_be_parsed_as(input: &str, rule: Rule) {
        let parsed = EmjayGrammar::parse(rule, input).unwrap().next().unwrap();
        assert_eq!(input, parsed.as_str());
    }

    #[test]
    fn grammar_can_parse_number() {
        assert_can_be_parsed_as("0", Rule::number);
        assert_can_be_parsed_as("1", Rule::number);
        assert_can_be_parsed_as("-123", Rule::number);
        assert_can_be_parsed_as("0x42A", Rule::number);
        assert_can_be_parsed_as("-0x42A", Rule::number);
    }

    #[test]
    fn grammar_can_parse_identifier() {
        assert_can_be_parsed_as("x", Rule::identifier);
        assert_can_be_parsed_as("x_32", Rule::identifier);
        assert_can_be_parsed_as("éñò", Rule::identifier);
    }

    #[test]
    fn grammar_can_parse_function_call() {
        assert_can_be_parsed_as("f()", Rule::functionCall);
        assert_can_be_parsed_as("f(1)", Rule::functionCall);
        assert_can_be_parsed_as("f(a, 3 + b)", Rule::functionCall);
    }

    #[test]
    fn grammar_can_parse_expression() {
        assert_can_be_parsed_as("x", Rule::expression);
        assert_can_be_parsed_as("42", Rule::expression);
        assert_can_be_parsed_as("-3", Rule::expression);
        assert_can_be_parsed_as("3 * 4 + g", Rule::expression);
        assert_can_be_parsed_as("-(1 + x) * 4 - f()", Rule::expression);
        assert_can_be_parsed_as("f()", Rule::expression);
    }

    #[test]
    fn grammar_can_parse_statement_let() {
        assert_can_be_parsed_as("let x = 1", Rule::letStatement);
        assert_can_be_parsed_as("let y_3π = 1 + x", Rule::letStatement);
    }

    #[test]
    fn grammar_can_parse_statement_assignment() {
        assert_can_be_parsed_as("x = x + 1", Rule::assignmentStatement);
    }

    #[test]
    fn grammar_can_parse_statement_return() {
        assert_can_be_parsed_as("return 42", Rule::returnStatement);
    }

    #[test]
    fn grammar_can_parse_block() {
        assert_can_be_parsed_as("{}", Rule::block);
        assert_can_be_parsed_as("{ x = y; }", Rule::block);
        assert_can_be_parsed_as("{ let x = y; { {} } let z = x; }", Rule::block);
    }

    #[test]
    fn grammar_can_parse_function() {
        assert_can_be_parsed_as("fn main() { let x = y; }", Rule::functionDeclaration);
        assert_can_be_parsed_as("fn f(a) {}", Rule::functionDeclaration);
        assert_can_be_parsed_as("fn f(a, b, c, d, e) {}", Rule::functionDeclaration);
    }

    #[test]
    fn grammar_can_parse_program() {
        assert_can_be_parsed_as("fn main() { }\nfn foo() { let x = 1; }", Rule::program);
    }
}
