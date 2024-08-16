use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct EmjayParser;

#[cfg(test)]
mod tests {
    use super::{EmjayParser, Rule};
    use pest::Parser;

    #[test]
    fn can_parse_let() {
        let parsed = EmjayParser::parse(Rule::letStatement, "let x1 = y")
            .unwrap()
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
}
