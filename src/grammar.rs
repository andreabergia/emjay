use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
#[allow(dead_code)]
pub struct EmjayGrammar;

#[cfg(test)]
mod tests {
    use super::{EmjayGrammar, Rule};
    use pest::Parser;

    fn parse_as(input: &str, rule: Rule) -> &str {
        let parsed = EmjayGrammar::parse(rule, input)
            .expect(&format!("can parse as {:?}", rule))
            .next()
            .unwrap();
        parsed.as_str()
    }

    #[test]
    fn grammar_can_parse_number() {
        assert_eq!("0", parse_as("0", Rule::number));
        assert_eq!("1", parse_as("1", Rule::number));
        assert_eq!("-123", parse_as("-123", Rule::number));
        assert_eq!("0.123", parse_as("0.123", Rule::number));
        assert_eq!("1e6", parse_as("1e6", Rule::number));
        assert_eq!("1.2e7", parse_as("1.2e7", Rule::number));
        assert_eq!("0x42A", parse_as("0x42A", Rule::number));
        assert_eq!("-0x42A", parse_as("-0x42A", Rule::number));
    }

    #[test]
    fn grammar_can_parse_identifier() {
        assert_eq!("x", parse_as("x", Rule::identifier));
        assert_eq!("x_32", parse_as("x_32", Rule::identifier));
        assert_eq!("éñò", parse_as("éñò", Rule::identifier));
    }

    #[test]
    fn grammar_can_parse_expression() {
        assert_eq!("x", parse_as("x", Rule::expression));
        assert_eq!("42", parse_as("42", Rule::expression));
        assert_eq!("-3", parse_as("-3", Rule::expression));
        assert_eq!("2!", parse_as("2!", Rule::expression));
        assert_eq!("3 * 4 + 2", parse_as("3 * 4 + 2", Rule::expression));
        assert_eq!(
            "-(1 + x) * 4 - 2",
            parse_as("-(1 + x) * 4 - 2", Rule::expression)
        );
    }

    #[test]
    fn grammar_can_parse_let() {
        let parsed = EmjayGrammar::parse(Rule::statement, "let x = 1;")
            .expect("can parse let statement")
            .next()
            .unwrap();
        if let Rule::letStatement = parsed.as_rule() {
            let mut inner = parsed.into_inner();
            let id = inner.next().unwrap().as_str();
            let expression = inner.next().unwrap().as_str();
            assert_eq!(id, "x");
            assert_eq!(expression, "1");
        } else {
            assert!(false, "should have parsed a let statement");
        }
    }

    #[test]
    fn grammar_can_parse_assignment() {
        let parsed = EmjayGrammar::parse(Rule::statement, "x = y;")
            .expect("can parse assignment statement")
            .next()
            .unwrap();
        if let Rule::assignmentStatement = parsed.as_rule() {
            let mut inner = parsed.into_inner();
            let id = inner.next().unwrap().as_str();
            let expression = inner.next().unwrap().as_str();
            assert_eq!(id, "x");
            assert_eq!(expression, "y");
        } else {
            assert!(false, "should have parsed an assignment statement");
        }
    }

    #[test]
    fn grammar_can_parse_empty_block() {
        let parsed = EmjayGrammar::parse(Rule::block, "{}")
            .expect("can parse empty block")
            .next()
            .unwrap();
        if let Rule::block = parsed.as_rule() {
            let mut inner = parsed.into_inner();
            assert!(inner.next().is_none());
        } else {
            assert!(false, "should have parsed a block");
        }
    }

    #[test]
    fn grammar_can_parse_function() {
        let parsed = EmjayGrammar::parse(Rule::functionDeclaration, "fn main() { let x = y; }")
            .expect("can parse function")
            .next()
            .unwrap();
        if let Rule::functionDeclaration = parsed.as_rule() {
            let mut inner = parsed.into_inner();
            let id = inner.next().unwrap().as_str();
            let block_as_str = inner.next().unwrap().as_str();
            assert_eq!(id, "main");
            assert_eq!(block_as_str, "{ let x = y; }");
        } else {
            assert!(false, "should have parsed a function");
        }
    }

    #[test]
    fn grammar_can_parse_program() {
        let parsed = EmjayGrammar::parse(
            Rule::program,
            r"
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
        assert!(
            matches!(parsed.as_rule(), Rule::program),
            "should have parsed a program"
        );
    }
}
