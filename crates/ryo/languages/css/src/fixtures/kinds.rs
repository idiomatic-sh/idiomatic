//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it.

use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample, SampleText, Why};

use crate::domain::extras::constants::DECODING;
use crate::domain::extras::service::content;
use crate::domain::extras::types::Content;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::constants::STRUCTURE;
use crate::domain::nodes::types::CssNode;
use crate::domain::syntax::constants::UNREAD_TEXT;
use crate::domain::syntax::service::{kind_bits, kind_name, kinds};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::constants::{BROKEN, COMPONENT, NEVER_MADE, PUNCTUATION, TRIVIA};
use crate::domain::tokens::types::Token;

/// Every kind, in the order of their numbers, with how the language shows it.
pub(super) fn shown() -> Vec<KindSample> {
    kinds()
        .map(|kind| KindSample {
            kind: KindName(kind_name(kind).0.into()),
            bits: kind_bits(kind),
            sample: sample(kind),
        })
        .collect()
}

/// How the language shows a node: the source whose tree holds it, or why no relation reads it.
const fn node(node: CssNode) -> Sample {
    match node {
        CssNode::Stylesheet => Sample::Node(SampleText("")),
        CssNode::AtRule => Sample::Node(SampleText("@import \"a.css\";")),
        CssNode::Function => Sample::Node(SampleText("a { b: url(\"c.png\") }")),
        CssNode::QualifiedRule | CssNode::Declaration | CssNode::SimpleBlock => {
            Sample::Unread(Why(STRUCTURE))
        }
    }
}

/// How the language shows a token: the source whose tree holds it, or why no relation reads it.
const fn token(token: Token) -> Sample {
    match token {
        Token::String => Sample::Token(SampleText("a { b: \"c\" }")),
        Token::Url => Sample::Token(SampleText("a { b: url(c.png) }")),
        Token::Ident
        | Token::Function
        | Token::AtKeyword
        | Token::Hash
        | Token::Delim
        | Token::Number
        | Token::Percentage
        | Token::Dimension => Sample::Unread(Why(COMPONENT)),
        Token::UnicodeRange => Sample::Unread(Why(NEVER_MADE)),
        Token::Whitespace => Sample::Unread(Why(TRIVIA)),
        Token::Cdo
        | Token::Cdc
        | Token::Colon
        | Token::Semicolon
        | Token::Comma
        | Token::LeftSquare
        | Token::RightSquare
        | Token::LeftParenthesis
        | Token::RightParenthesis
        | Token::LeftCurly
        | Token::RightCurly => Sample::Unread(Why(PUNCTUATION)),
        Token::BadString | Token::BadUrl => Sample::Unread(Why(BROKEN)),
    }
}

/// How the language shows an extra: a comment by its source, and why no relation reads the others.
const fn extra(extra: Extra) -> Sample {
    match (extra, content(extra)) {
        (Extra::Comment, Content::Read) => Sample::Token(SampleText("/* a */")),
        (Extra::ByteOrderMark, _) => Sample::Unread(Why(DECODING)),
        (Extra::Error, _) | (Extra::Comment, Content::Kept) => Sample::Unread(Why(UNREAD_TEXT)),
    }
}

/// How the language shows a kind of its trees: the source whose tree holds it, for a kind a relation reads, and for every other the
/// reason no relation models it. Each match has no wildcard arm, so a kind the standard gains is a compile error until it is shown.
#[must_use]
pub(crate) const fn sample(kind: Kind) -> Sample {
    match kind {
        Kind::Node(each) => node(each),
        Kind::Token(each) => token(each),
        Kind::Extra(each) => extra(each),
    }
}
