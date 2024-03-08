use std::num::ParseFloatError;

use logos::Logos;

#[derive(Default, Debug, Clone, PartialEq)]
enum LexingError {
    InvalidFloat,
    #[default]
    UnrecognizedToken,
}

impl From<ParseFloatError> for LexingError {
    fn from(_: ParseFloatError) -> Self {
        Self::InvalidFloat
    }
}

#[derive(Debug, PartialEq, Logos)]
#[logos(skip r"[ \t]+")]
#[logos(error = LexingError)]
enum Token<'source> {
    #[regex(r"not")]
    Not,
    #[regex(r"and")]
    And,
    #[regex(r"or")]
    Or,
    #[regex(r"xor")]
    Xor,
    #[regex(r"if")]
    If,
    #[regex(r"else")]
    Else,

    #[regex(r"\+")]
    Plus,
    #[regex(r"-")]
    Minus,
    #[regex(r"\*")]
    Multiply,
    #[regex(r"/")]
    Divide,

    #[regex(r"\.")]
    Dot,
    #[regex(r"=")]
    Equal,

    #[regex(r"==")]
    DoubleEqual,
    #[regex(r"!=")]
    Different,
    #[regex(r"<")]
    Lesser,
    #[regex(r"<=")]
    LesserOrEqual,
    #[regex(r">")]
    Greater,
    #[regex(r">=")]
    GreaterOrEqual,

    #[regex(r"\(")]
    OpenParenthesis,
    #[regex(r"\)")]
    CloseParenthesis,
    #[regex(r"\[")]
    OpenBracket,
    #[regex(r"\]")]
    CloseBracket,
    #[regex(r"\{")]
    OpenBrace,
    #[regex(r"\}")]
    CloseBrace,

    #[regex(r"(0|[1-9][0-9]*)(\.[0-9]*)?", |lex| lex.slice().parse())]
    FloatNumber(f64),

    #[regex(r"[a-zA-Z][a-zA-Z0-9_]*")]
    Identifier(&'source str),
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
    fn lex_keywords() {
        check_lex_one_token("not", Token::Not);
        check_lex_one_token("and", Token::And);
        check_lex_one_token("or", Token::Or);
        check_lex_one_token("xor", Token::Xor);
        check_lex_one_token("if", Token::If);
        check_lex_one_token("else", Token::Else);
    }

    #[test]
    fn lex_operators() {
        check_lex_one_token("+", Token::Plus);
        check_lex_one_token("-", Token::Minus);
        check_lex_one_token("*", Token::Multiply);
        check_lex_one_token("/", Token::Divide);

        check_lex_one_token(".", Token::Dot);
        check_lex_one_token("=", Token::Equal);

        check_lex_one_token("==", Token::DoubleEqual);
        check_lex_one_token("!=", Token::Different);
        check_lex_one_token("<", Token::Lesser);
        check_lex_one_token("<=", Token::LesserOrEqual);
        check_lex_one_token(">", Token::Greater);
        check_lex_one_token(">=", Token::GreaterOrEqual);

        check_lex_one_token("(", Token::OpenParenthesis);
        check_lex_one_token(")", Token::CloseParenthesis);
        check_lex_one_token("[", Token::OpenBracket);
        check_lex_one_token("]", Token::CloseBracket);
        check_lex_one_token("{", Token::OpenBrace);
        check_lex_one_token("}", Token::CloseBrace);
    }

    #[test]
    fn lex_numbers() {
        check_lex_one_token("0", Token::FloatNumber(0.0));
        check_lex_one_token("42", Token::FloatNumber(42.0));
        check_lex_one_token("0.1", Token::FloatNumber(0.1));
        check_lex_one_token("1.", Token::FloatNumber(1.0));
        check_lex_one_token("123.456", Token::FloatNumber(123.456));
    }

    #[test]
    fn lex_identifier() {
        check_lex_one_token("alpha", Token::Identifier("alpha"));
        check_lex_one_token("a_name", Token::Identifier("a_name"));
        check_lex_one_token("aName123", Token::Identifier("aName123"));
    }

    #[test]
    fn lex_token_sequence() {
        let mut lex = Token::lexer("3 + alpha");

        assert_eq!(lex.next(), Some(Ok(Token::FloatNumber(3.0))));
        assert_eq!(lex.slice(), "3");
        assert_eq!(lex.span(), 0..1);

        assert_eq!(lex.next(), Some(Ok(Token::Plus)));
        assert_eq!(lex.slice(), "+");
        assert_eq!(lex.span(), 2..3);

        assert_eq!(lex.next(), Some(Ok(Token::Identifier("alpha"))));
        assert_eq!(lex.slice(), "alpha");
        assert_eq!(lex.span(), 4..9);

        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_error() {
        let mut lex = Token::lexer("_");

        assert_eq!(lex.next(), Some(Err(LexingError::UnrecognizedToken)));
        assert_eq!(lex.slice(), "_");
        assert_eq!(lex.span(), 0..1);
    }
}
