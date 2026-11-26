//! Lazily intern bottom-up parsed expression keys; source formatting and runtime value equality are separate.
use crate::domain::{tokens::types::Token};


/// Operators are meaningful; separators, redundant parentheses and lexical trivia are not.
pub(crate) fn significant(token: Token) -> Option<Token> {
    matches!(
        token,
        Token::Name
            | Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Vbar
            | Token::Amper
            | Token::Less
            | Token::Greater
            | Token::Percent
            | Token::Eqequal
            | Token::Notequal
            | Token::Lessequal
            | Token::Greaterequal
            | Token::Tilde
            | Token::Circumflex
            | Token::Leftshift
            | Token::Rightshift
            | Token::Doublestar
            | Token::Doubleslash
            | Token::At
            | Token::Ellipsis
    )
    .then_some(token)
}
