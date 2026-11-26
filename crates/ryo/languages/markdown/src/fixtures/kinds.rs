//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample, SampleText, Why};

use crate::domain::syntax::constants::{
    EXTENSION_OFF, MARKUP, PROSE, SECTION, STRUCTURE, THROUGH_PARENT, TRIVIA, UNREAD_TEXT,
};
use crate::domain::syntax::service::{kind_bits, kind_name, kinds};
use crate::domain::syntax::types::Kind;

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

/// How the language shows a kind of its trees: the source whose tree holds it, for a kind a relation reads, and for every other the
/// reason no relation models it. The match has no wildcard arm, so a kind the
/// grammar gains is a compile error until it is shown.
#[must_use]
pub(crate) const fn sample(kind: Kind) -> Sample {
    match kind {
        Kind::Node(Node::Root) => Sample::Node(SampleText("")),
        Kind::Node(Node::Html) => Sample::Node(SampleText("<!-- a -->\n")),
        Kind::Node(Node::InlineCode) => Sample::Token(SampleText("`a`\n")),
        Kind::Node(Node::Code) => Sample::Node(SampleText("```\na\n```\n")),
        Kind::Node(Node::Paragraph) => Sample::Node(SampleText("a\n")),
        Kind::Node(Node::Heading) => Sample::Node(SampleText("# a\n")),
        Kind::Node(Node::ListItem) => Sample::Node(SampleText("- a\n")),
        Kind::Node(Node::TableCell) => Sample::Node(SampleText("| a |\n|---|\n| b |\n")),
        Kind::Node(Node::Link) => Sample::Node(SampleText("[a](b)\n")),
        Kind::Node(Node::LinkReference) => Sample::Node(SampleText("[a][b]\n\n[b]: c\n")),
        Kind::Node(Node::Image) => Sample::Node(SampleText("![a](b)\n")),
        Kind::Node(Node::ImageReference) => Sample::Node(SampleText("![a][b]\n\n[b]: c\n")),
        Kind::Node(Node::FootnoteDefinition) => Sample::Node(SampleText("a[^1]\n\n[^1]: b\n")),
        Kind::Node(Node::Definition) => Sample::Token(SampleText("[a]\n\n[a]: b\n")),
        Kind::Node(
            Node::Blockquote
            | Node::List
            | Node::Table
            | Node::TableRow
            | Node::Emphasis
            | Node::Strong
            | Node::Delete
            | Node::FootnoteReference
            | Node::ThematicBreak,
        )
        | Kind::Extra(
            Extra::Yaml
            | Extra::Toml
            | Extra::InlineMath
            | Extra::Math
            | Extra::TaskChecked
            | Extra::TaskUnchecked,
        ) => Sample::Unread(Why(STRUCTURE)),
        Kind::Extra(
            Extra::DefinitionList
            | Extra::DefinitionListTitle
            | Extra::DefinitionListDefinition
            | Extra::Superscript
            | Extra::Subscript,
        ) => Sample::Unread(Why(EXTENSION_OFF)),
        Kind::Node(Node::Text | Node::Break) | Kind::Token(Token::LineEnding) => {
            Sample::Unread(Why(PROSE))
        }
        Kind::Token(Token::CodeFlowValue | Token::HtmlFlowData) => {
            Sample::Unread(Why(THROUGH_PARENT))
        }
        Kind::Extra(Extra::Markup) => Sample::Unread(Why(MARKUP)),
        Kind::Extra(Extra::Section) => Sample::Unread(Why(SECTION)),
        Kind::Token(Token::Whitespace) => Sample::Unread(Why(TRIVIA)),
        Kind::Extra(Extra::Error) => Sample::Unread(Why(UNREAD_TEXT)),
    }
}
