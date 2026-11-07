//! The `tokens` service: free functions named by verb over the domain's types, with no IO and no ports. It gives an opening token's
//! mirror, the token that closes the block it opens.

use super::types::Token;

/// The mirror of an opening token (§5.3): the `)-token` of a `(-token` or a `function-token`, the `]-token` of a `[-token`, the
/// `}-token` of a `{-token`; nothing for any other token.
#[must_use]
pub(crate) const fn mirror(token: Token) -> Option<Token> {
    match token {
        Token::Function | Token::LeftParenthesis => Some(Token::RightParenthesis),
        Token::LeftSquare => Some(Token::RightSquare),
        Token::LeftCurly => Some(Token::RightCurly),
        Token::Ident
        | Token::AtKeyword
        | Token::Hash
        | Token::String
        | Token::BadString
        | Token::Url
        | Token::BadUrl
        | Token::Delim
        | Token::Number
        | Token::Percentage
        | Token::Dimension
        | Token::UnicodeRange
        | Token::Whitespace
        | Token::Cdo
        | Token::Cdc
        | Token::Colon
        | Token::Semicolon
        | Token::Comma
        | Token::RightSquare
        | Token::RightParenthesis
        | Token::RightCurly => None,
    }
}
