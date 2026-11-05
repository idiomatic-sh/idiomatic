//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it.

use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample, SampleText, Why};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::{PUNCTUATION, THROUGH_PARTS, TRIVIA, UNREAD_TEXT};
use crate::domain::syntax::service::{kind_bits, kinds};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// Every kind, in the order of their numbers, with how the language shows it.
pub(super) fn shown() -> Vec<KindSample> {
    kinds()
        .map(|kind| KindSample {
            kind: KindName(format!("{kind:?}").into()),
            bits: kind_bits(kind),
            sample: sample(kind),
        })
        .collect()
}

/// How the language shows a kind of its trees: the source whose tree holds it, for a kind a relation reads, and for every other the
/// reason no relation models it. The match has no wildcard arm, so a kind the
/// grammar gains is a compile error until it is shown.
#[must_use]
pub(crate) const fn sample(kind: Kind) -> Sample {
    match kind {
        Kind::Node(Node::Toml) => Sample::Node(SampleText("")),
        Kind::Node(Node::StdTable) => Sample::Node(SampleText("[package]")),
        Kind::Node(Node::ArrayTable) => Sample::Node(SampleText("[[bin]]")),
        Kind::Node(Node::Keyval | Node::Key) => Sample::Unread(Why(THROUGH_PARTS)),
        Kind::Node(Node::Array) => Sample::Node(SampleText("a = [1]")),
        Kind::Node(Node::InlineTable) => Sample::Node(SampleText("a = {}")),
        Kind::Token(Token::UnquotedKey | Token::Integer) => Sample::Token(SampleText("a = 1")),
        Kind::Token(Token::BasicString) => Sample::Token(SampleText("a = \"x\"")),
        Kind::Token(Token::LiteralString) => Sample::Token(SampleText("a = 'x'")),
        Kind::Token(Token::MlBasicString) => Sample::Token(SampleText("a = \"\"\"x\"\"\"")),
        Kind::Token(Token::MlLiteralString) => Sample::Token(SampleText("a = '''x'''")),
        Kind::Token(Token::Float) => Sample::Token(SampleText("a = 1.5")),
        Kind::Token(Token::Boolean) => Sample::Token(SampleText("a = true")),
        Kind::Token(Token::DateTime) => Sample::Token(SampleText("a = 1979-05-27")),
        Kind::Token(Token::Comment) => Sample::Token(SampleText("# a")),
        Kind::Token(Token::Ws | Token::Newline) => Sample::Unread(Why(TRIVIA)),
        Kind::Token(
            Token::KeyvalSep
            | Token::DotSep
            | Token::ArraySep
            | Token::InlineTableSep
            | Token::StdTableOpen
            | Token::StdTableClose
            | Token::ArrayOpen
            | Token::ArrayClose
            | Token::ArrayTableOpen
            | Token::ArrayTableClose
            | Token::InlineTableOpen
            | Token::InlineTableClose,
        ) => Sample::Unread(Why(PUNCTUATION)),
        Kind::Extra(Extra::Error) => Sample::Unread(Why(UNREAD_TEXT)),
    }
}
