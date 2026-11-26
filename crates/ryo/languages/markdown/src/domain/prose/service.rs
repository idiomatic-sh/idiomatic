//! The `prose` service: free functions named by verb that read the text of a run of a document, with no IO and no ports. A run is a
//! paragraph, the title of a heading, a cell of a table, or the text an item of a list holds itself. Its markup is joined away, its code,
//! math and raw HTML are masked by a space, a break is a space, and the white space between words is one space.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

use super::constants::{CODE_FENCE, WORD_SPACE};
use super::types::{Code, Inline, Spoken, WordGap, Wording};
use crate::domain::syntax::types::Kind;

/// What an inline element of the kind `of` adds to a run.
#[must_use]
pub(crate) fn inline_of(of: Kind) -> Inline {
    if of == Kind::Node(Node::Text) {
        Inline::Words
    } else if matches!(
        of,
        Kind::Token(Token::LineEnding | Token::HtmlFlowData)
            | Kind::Node(Node::Break | Node::InlineCode | Node::Html)
            | Kind::Extra(Extra::InlineMath)
    ) {
        Inline::Mask
    } else if matches!(
        of,
        Kind::Node(
            Node::Emphasis
                | Node::Strong
                | Node::Delete
                | Node::Link
                | Node::LinkReference
                | Node::Image
                | Node::ImageReference
        ) | Kind::Extra(Extra::Superscript | Extra::Subscript)
    ) {
        Inline::Holds
    } else {
        Inline::Nothing
    }
}
