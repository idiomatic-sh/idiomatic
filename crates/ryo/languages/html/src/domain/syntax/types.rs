//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. A kind of node or token is a node, a token or an extra, named in kebab case; the element table holds its number.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

/// What a node or a token of a html syntax tree is: a node or a token of the standard, or an extra the standard lacks. The element table
/// holds its number: the nodes first, then the tokens, then the extras, each in the order of its enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    /// A node.
    Node(Node),
    /// A token.
    Token(Token),
    /// A kind the standard lacks.
    Extra(Extra),
}

/// Whether a syntax position is the document root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rootness {
    /// Document root.
    Root,
    /// A descendant.
    Inner,
}

/// Native DOM scalar properties, distinct from source tokens and child edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Property {
    /// Local name.
    Name,
    /// Namespace URI.
    Namespace,
    /// Namespace prefix.
    Prefix,
    /// Decoded text or attribute value.
    Value,
    /// Doctype public identifier.
    PublicId,
    /// Doctype system identifier.
    SystemId,
    /// Document quirks mode.
    DocumentMode,
}

/// The role of an element under its parent: a DOM scalar property's node is in the role of its property, an attribute in the role
/// `attribute` under its element, and every other edge has none. `syntax_child.field` names it.
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
    /// An attribute of an element.
    Attribute,
    /// A local name.
    Name,
    /// A namespace URI.
    Namespace,
    /// A namespace prefix.
    Prefix,
    /// A decoded value.
    Value,
    /// A doctype's public identifier.
    PublicId,
    /// A doctype's system identifier.
    SystemId,
    /// The document's quirks mode.
    DocumentMode,
}

/// The result of the section classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Section {
    /// The matching form.
    Section,
    /// Any other form.
    Other,
}

/// What an element or attribute defines: a titled section, named by the heading that is its title, or an anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Defined {
    /// A sectioning element with a heading: the heading's element.
    Section(ryo::languages::Position),
    /// An `id`, or the `name` of an anchor element.
    Label,
    /// Nothing.
    Nothing,
}

/// The result of the readability classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Readability {
    /// The matching form.
    Prose,
    /// Any other form.
    Excluded,
}

/// Whether an element continues the run of text around it or holds a run of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flow {
    /// The element is part of the run around it (a link, emphasis).
    Inline,
    /// The element holds a run of its own (a paragraph, a heading, a cell).
    Block,
}

/// The result of the anchor classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Anchor {
    /// The matching form.
    Definition,
    /// Any other form.
    Other,
}

/// The result of the reference classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reference {
    /// The matching form.
    Url,
    /// Any other form.
    Other,
}

/// Whether a title has a word yet: the next word is a space after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wording {
    /// No word yet.
    Silent,
    /// A word.
    Words,
}
