//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it.

use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample, SampleText, Why};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::JsonNode;
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
        Kind::Node(JsonNode::JsonText) => Sample::Node(SampleText("")),
        Kind::Node(JsonNode::Object) => Sample::Node(SampleText("{}")),
        Kind::Node(JsonNode::Array) => Sample::Node(SampleText("[]")),
        Kind::Node(JsonNode::Member) => Sample::Unread(Why(THROUGH_PARTS)),
        Kind::Token(Token::String) => Sample::Token(SampleText("\"a\"")),
        Kind::Token(Token::Number) => Sample::Token(SampleText("1")),
        Kind::Token(Token::True) => Sample::Token(SampleText("true")),
        Kind::Token(Token::False) => Sample::Token(SampleText("false")),
        Kind::Token(Token::Null) => Sample::Token(SampleText("null")),
        Kind::Token(
            Token::BeginArray
            | Token::BeginObject
            | Token::EndArray
            | Token::EndObject
            | Token::NameSeparator
            | Token::ValueSeparator,
        ) => Sample::Unread(Why(PUNCTUATION)),
        Kind::Token(Token::Ws) => Sample::Unread(Why(TRIVIA)),
        Kind::Extra(Extra::LineComment) => Sample::Token(SampleText("// c\n")),
        Kind::Extra(Extra::BlockComment) => Sample::Token(SampleText("/* c */")),
        Kind::Extra(Extra::Error) => Sample::Unread(Why(UNREAD_TEXT)),
    }
}
