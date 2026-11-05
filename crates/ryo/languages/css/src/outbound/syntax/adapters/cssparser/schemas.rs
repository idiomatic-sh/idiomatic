//! What the parser gives, which the adapter does not control, and its conversion into the language's kinds: the one declaration of
//! how each of the library's tokens is a kind of the language's trees, which text of it the library decoded, and what it says of an
//! error.

use std::fmt::Write as _;

use cssparser::{ParseErrorKind, Token};

use super::constants::{
    BAD_STRING, BAD_URL, REFUSED, UNMATCHED_BRACE, UNMATCHED_BRACKET, UNMATCHED_PARENTHESIS,
};
use super::types::{Met, Pieces, Scratch};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::CssNode;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token as Lexeme;
use ryo::languages::{NameText, ReportText};

/// A token of the library.
pub(super) struct TokenSchema<'a, 'i>(pub &'a Token<'i>);

impl From<TokenSchema<'_, '_>> for Kind {
    fn from(schema: TokenSchema<'_, '_>) -> Self {
        Self::Token(match schema.0 {
            Token::Ident(_) => Lexeme::Ident,
            Token::AtKeyword(_) => Lexeme::AtKeyword,
            Token::Hash(_) | Token::IDHash(_) => Lexeme::Hash,
            Token::QuotedString(_) => Lexeme::String,
            Token::UnquotedUrl(_) => Lexeme::Url,
            // The standard has no token for `~=`, `|=`, `^=`, `$=` and `*=`: each is two delim-tokens (`pieces`).
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
            // The standard's tokenizer consumes a comment and outputs no token: the tree keeps it as an extra.
            Token::Comment(_) => return Self::Extra(Extra::Comment),
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
}

/// How many of the standard's tokens a token of the library is: one, or the two delim-tokens of an attribute matcher.
pub(super) fn pieces(token: &Token<'_>) -> Pieces {
    match token {
        Token::IncludeMatch
        | Token::DashMatch
        | Token::PrefixMatch
        | Token::SuffixMatch
        | Token::SubstringMatch => Pieces::TwoDelims,
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
        | Token::CDO
        | Token::CDC
        | Token::Function(_)
        | Token::ParenthesisBlock
        | Token::SquareBracketBlock
        | Token::CurlyBracketBlock
        | Token::BadUrl(_)
        | Token::BadString(_)
        | Token::CloseParenthesis
        | Token::CloseSquareBracket
        | Token::CloseCurlyBracket => Pieces::Whole,
    }
}

/// The language's kind for a token of the library.
pub(super) fn token_kind(token: &Token<'_>) -> Kind {
    Kind::from(TokenSchema(token))
}

/// The text the library decoded from a token that the readers read it of, its escapes resolved: a string without its quotes, a
/// comment without its `/*` and `*/`; nothing for any other token (a url's is the target the adapter resolves it to).
pub(super) fn decoded_text<'a>(token: &'a Token<'_>) -> Option<NameText<'a>> {
    match token {
        Token::QuotedString(text) => Some(NameText(text)),
        Token::Comment(text) => Some(NameText(text)),
        Token::Ident(_)
        | Token::AtKeyword(_)
        | Token::Function(_)
        | Token::UnquotedUrl(_)
        | Token::Hash(_)
        | Token::IDHash(_)
        | Token::Delim(_)
        | Token::Number { .. }
        | Token::Percentage { .. }
        | Token::Dimension { .. }
        | Token::WhiteSpace(_)
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
        | Token::ParenthesisBlock
        | Token::SquareBracketBlock
        | Token::CurlyBracketBlock
        | Token::BadUrl(_)
        | Token::BadString(_)
        | Token::CloseParenthesis
        | Token::CloseSquareBracket
        | Token::CloseCurlyBracket => None,
    }
}

/// What the adapter makes of a token the parser gives as it walks a run: the node a function or a simple block opens, what the
/// language says of a token the library calls a parse error (`Token::is_parse_error`), white space, or content.
pub(super) fn met(token: &Token<'_>) -> Met {
    match token {
        Token::WhiteSpace(_) | Token::Comment(_) => Met::Trivia,
        Token::Function(_) => Met::Opens(CssNode::Function),
        Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
            Met::Opens(CssNode::SimpleBlock)
        }
        Token::BadUrl(_) => Met::Broken(ReportText(BAD_URL)),
        Token::BadString(_) => Met::Broken(ReportText(BAD_STRING)),
        Token::CloseParenthesis => Met::Broken(ReportText(UNMATCHED_PARENTHESIS)),
        Token::CloseSquareBracket => Met::Broken(ReportText(UNMATCHED_BRACKET)),
        Token::CloseCurlyBracket => Met::Broken(ReportText(UNMATCHED_BRACE)),
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
        | Token::Colon
        | Token::Semicolon
        | Token::Comma
        | Token::IncludeMatch
        | Token::DashMatch
        | Token::PrefixMatch
        | Token::SuffixMatch
        | Token::SubstringMatch
        | Token::CDO
        | Token::CDC => Met::Content,
    }
}

/// Writes what the library says of an error of its rule parsers into `text`, cleared first.
pub(super) fn error_message(kind: &ParseErrorKind<()>, text: &mut Scratch) {
    text.0.clear();
    match kind {
        ParseErrorKind::Basic(basic) => {
            // Writing into a `String` fails only when the library's `Display` does: the message is then the refusal's.
            if write!(text.0, "{basic}").is_err() {
                text.0.clear();
                text.0.push_str(REFUSED);
            }
        }
        ParseErrorKind::Custom(()) => text.0.push_str(REFUSED),
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
