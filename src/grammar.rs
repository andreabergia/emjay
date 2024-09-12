use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
#[allow(dead_code)]
pub struct EmjayGrammar;

#[cfg(test)]
mod tests {
    use super::{EmjayGrammar, Rule};
    use pest::Parser;

    #[test]
    fn grammar_can_parse_let() {
        let parsed = EmjayGrammar::parse(Rule::letStatement, "let x1 = y")
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
        let p = EmjayGrammar::parse(
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
}
