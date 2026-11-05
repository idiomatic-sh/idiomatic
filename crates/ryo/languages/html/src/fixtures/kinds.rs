//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it.

use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample, SampleText, Why};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::HtmlNode;
use crate::domain::syntax::constants::{
    DOM_STRUCTURE, LEXICAL_STRUCTURE, PROPERTY_STRUCTURE, TEXT_RUN, UNREAD_TEXT,
};
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
        Kind::Node(HtmlNode::SourceFile | HtmlNode::Element) => Sample::Node(SampleText("")),
        Kind::Node(HtmlNode::Document | HtmlNode::DocumentType | HtmlNode::DocumentFragment) => {
            Sample::Unread(Why(DOM_STRUCTURE))
        }
        Kind::Node(HtmlNode::Attribute) => Sample::Node(SampleText("<p title='a'>")),
        Kind::Node(HtmlNode::Text) => Sample::Unread(Why(TEXT_RUN)),
        Kind::Node(HtmlNode::Comment) => Sample::Node(SampleText("<!-- a -->")),
        Kind::Node(
            HtmlNode::Name
            | HtmlNode::Namespace
            | HtmlNode::Prefix
            | HtmlNode::Value
            | HtmlNode::PublicId
            | HtmlNode::SystemId
            | HtmlNode::DocumentMode,
        ) => Sample::Unread(Why(PROPERTY_STRUCTURE)),
        Kind::Token(
            Token::Characters | Token::Doctype | Token::StartTag | Token::EndTag | Token::Comment,
        ) => Sample::Unread(Why(LEXICAL_STRUCTURE)),
        Kind::Extra(Extra::Error) => Sample::Unread(Why(UNREAD_TEXT)),
    }
}
