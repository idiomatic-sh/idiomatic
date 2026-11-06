//! Part of the walk of the `cssparser` fixtures: place, token_kind, is_trivia and more.

use super::super::types::ByteIndex;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token as Lexeme;
use cssparser::{Parser, Token};

/// Where the parser is, in bytes from the start of its text.
pub(super) fn place(input: &Parser<'_>) -> ByteIndex {
    ByteIndex(input.position().byte_index())
}

/// The standard's token of the library's, or the extra a comment is.
pub(super) fn token_kind(token: &Token<'_>) -> Kind {
    Kind::Token(match token {
        Token::Ident(_) => Lexeme::Ident,
        Token::AtKeyword(_) => Lexeme::AtKeyword,
        Token::Hash(_) | Token::IDHash(_) => Lexeme::Hash,
        Token::QuotedString(_) => Lexeme::String,
        Token::UnquotedUrl(_) => Lexeme::Url,
        Token::Delim(_)
        | Token::IncludeMatch
        | Token::DashMatch
        | Token::PrefixMatch
        | Token::SuffixMatch
        | Token::SubstringMatch => Lexeme::Delim,
        Token::Number { .. } => Lexeme::Number,
        Token::Percentage { .. } => Lexeme::Percentage,
        Token::Dimension { .. } => Lexeme::Dimension,
        Token::WhiteSpace(_) => Lexeme::Whitespace,
        Token::Comment(_) => return Kind::Extra(Extra::Comment),
        Token::Colon => Lexeme::Colon,
        Token::Semicolon => Lexeme::Semicolon,
        Token::Comma => Lexeme::Comma,
        Token::CDO => Lexeme::Cdo,
        Token::CDC => Lexeme::Cdc,
        Token::Function(_) => Lexeme::Function,
        Token::ParenthesisBlock => Lexeme::LeftParenthesis,
        Token::SquareBracketBlock => Lexeme::LeftSquare,
        Token::CurlyBracketBlock => Lexeme::LeftCurly,
        Token::BadUrl(_) => Lexeme::BadUrl,
        Token::BadString(_) => Lexeme::BadString,
        Token::CloseParenthesis => Lexeme::RightParenthesis,
        Token::CloseSquareBracket => Lexeme::RightSquare,
        Token::CloseCurlyBracket => Lexeme::RightCurly,
    })
}

/// Whether the token is white space or a comment.
pub(super) const fn is_trivia(token: &Token<'_>) -> bool {
    matches!(token, Token::WhiteSpace(_) | Token::Comment(_))
}

/// The node a token opens: a function or a simple block.
pub(super) const fn opens(token: &Token<'_>) -> Option<Node> {
    match token {
        Token::Function(_) => Some(Node::Function),
        Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
            Some(Node::SimpleBlock)
        }
        Token::Ident(_)
        | Token::AtKeyword(_)
        | Token::Hash(_)
        | Token::IDHash(_)
        | Token::QuotedString(_)
        | Token::UnquotedUrl(_)
        | Token::Delim(_)
        | Token::Number { .. }
        | Token::Percentage { .. }
        | Token::Dimension { .. }
        | Token::WhiteSpace(_)
        | Token::Comment(_)
        | Token::Colon
        | Token::Semicolon
        | Token::Comma
        | Token::IncludeMatch
        | Token::DashMatch
        | Token::PrefixMatch
        | Token::SuffixMatch
        | Token::SubstringMatch
        | Token::CDO
        | Token::CDC
        | Token::BadUrl(_)
        | Token::BadString(_)
        | Token::CloseParenthesis
        | Token::CloseSquareBracket
        | Token::CloseCurlyBracket => None,
    }
}

/// Whether the library reads the token as one where the standard reads two delim-tokens: an attribute matcher.
pub(super) const fn is_matcher(token: &Token<'_>) -> bool {
    matches!(
        token,
        Token::IncludeMatch
            | Token::DashMatch
            | Token::PrefixMatch
            | Token::SuffixMatch
            | Token::SubstringMatch
    )
}
