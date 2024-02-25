use std::num::ParseFloatError;

use logos::Logos;

#[derive(Default, Debug, Clone, PartialEq)]
enum LexingError {
    InvalidFloat,
    #[default]
    UnrecognizedToken,
}

impl From<ParseFloatError> for LexingError {
    fn from(err: ParseFloatError) -> Self {
        Self::InvalidFloat
    }
}

#[derive(Debug, PartialEq, Logos)]
#[logos(skip r"[ \t]+")]
#[logos(error = LexingError)]
enum Token<'source> {
    #[regex(r"0|[1-9][0-9]*", |lex| lex.slice().parse())]
    FloatNumber(f64),

    #[regex(r"[a-zA-Z][a-zA-Z0-9_]*")]
    Identifier(&'source str),

    #[regex(r"\+")]
    PlusOperator,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_lex_one_token(input: &str, expected: Token) {
        let mut lex = Token::lexer(input);

        assert_eq!(lex.next(), Some(Ok(expected)));
        assert_eq!(lex.slice(), input);
        assert_eq!(lex.span(), 0..input.len());

        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_numbers() {
        check_lex_one_token("42", Token::FloatNumber(42.0));
    }

    #[test]
    fn lex_identifier() {
        check_lex_one_token("alpha", Token::Identifier("alpha"));
    }

    #[test]
    fn lex_plus_operator() {
        check_lex_one_token("+", Token::PlusOperator);
    }

    #[test]
    fn lex_error() {
        let mut lex = Token::lexer("_");

        assert_eq!(lex.next(), Some(Err(LexingError::UnrecognizedToken)));
        assert_eq!(lex.slice(), "_");
        assert_eq!(lex.span(), 0..1);
    }
}
