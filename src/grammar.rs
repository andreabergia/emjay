use std::sync::{LazyLock, OnceLock};

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
        .op(Op::postfix(Rule::fac))
        .op(Op::prefix(Rule::neg))
});

pub fn pratt_parser() -> &'static PrattParser<Rule> {
    &*EMJAY_PRATT_PARSER
}

#[cfg(test)]
mod tests {
    use super::{EmjayGrammar, Rule};
    use pest::Parser;

    fn assert_can_be_parsed_as(input: &str, rule: Rule) {
        let parsed = EmjayGrammar::parse(rule, input)
            .expect(&format!("can parse as {:?}", rule))
            .next()
            .unwrap();
        assert_eq!(input, parsed.as_str());
    }

    #[test]
    fn grammar_can_parse_number() {
        assert_can_be_parsed_as("0", Rule::number);
        assert_can_be_parsed_as("1", Rule::number);
        assert_can_be_parsed_as("-123", Rule::number);
        assert_can_be_parsed_as("0.123", Rule::number);
        assert_can_be_parsed_as("1e6", Rule::number);
        assert_can_be_parsed_as("1.2e7", Rule::number);
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
    fn grammar_can_parse_expression() {
        assert_can_be_parsed_as("x", Rule::expression);
        assert_can_be_parsed_as("42", Rule::expression);
        assert_can_be_parsed_as("-3", Rule::expression);
        assert_can_be_parsed_as("2!", Rule::expression);
        assert_can_be_parsed_as("3 * 4 + 2", Rule::expression);
        assert_can_be_parsed_as("-(1 + x) * 4 - 2", Rule::expression);
    }

    #[test]
    fn grammar_can_parse_let() {
        assert_can_be_parsed_as("let x = 1", Rule::letStatement);
        assert_can_be_parsed_as("let y_3π = 1 + x", Rule::letStatement);
    }

    #[test]
    fn grammar_can_parse_assignment() {
        assert_can_be_parsed_as("x = x + 1", Rule::assignmentStatement);
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
    }

    #[test]
    fn grammar_can_parse_program() {
        assert_can_be_parsed_as("fn main() { }\nfn foo() { let x = 1; }", Rule::program);
    }
}
