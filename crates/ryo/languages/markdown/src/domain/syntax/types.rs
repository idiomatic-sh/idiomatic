//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. A kind of node or token is a node, a token or an extra, named in kebab case (`heading`,
//! `code`, `inline-code`); the element table holds its number.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::MarkdownNode;
use crate::domain::tokens::types::Token;

/// What a node or a token of a Markdown syntax tree is: a node of mdast, a token of micromark, or an extra the standards lack. The element
/// table holds its number: the nodes first, then the tokens, then the extras, each in the order of its enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    /// A node of mdast.
    Node(MarkdownNode),
    /// A token of micromark.
    Token(Token),
    /// A kind the standards lack.
    Extra(Extra),
}

/// Whether an element is a token, a leaf, or a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Leaf {
    /// A token.
    Token,
    /// A node.
    Node,
}

/// Whether an element is the root of its tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rootness {
    /// The root.
    Root,
    /// Anything below it.
    Inner,
}

/// Whether raw HTML is a comment, `<!-- a -->`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Commentness {
    /// It is a comment.
    Comment,
    /// It is other HTML.
    Markup,
}

/// Text of the file, as it is written: what an element spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Text<'a>(pub &'a str);

/// How deep a heading is: one for `#`, up to six for `######`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Level(pub u8);

/// The role of an element under its parent, named by the property of mdast's node that holds it: `children`, the content of a
/// parent (`Parent.children`), `value`, a line of a literal's text (`Literal.value` of `code` and `html`), and `checked`, the mark
/// of a task (`ListItem.checked`, GFM). Every other edge has none: the markers, indentation, fences and destinations the parser gives
/// as markup, white space, a line ending a paragraph holds in its text, and a section, which mdast has no node for.
/// `syntax_child.field` names it.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
pub(crate) enum Field {
    /// The edge has no role.
    #[default]
    None,
    /// A node of a parent's content: a block of a section or a list item, a span of a paragraph.
    Children,
    /// A line of the text of a code block or a block of raw HTML.
    Value,
    /// The mark of a task, `[x]` or `[ ]`.
    Checked,
}
