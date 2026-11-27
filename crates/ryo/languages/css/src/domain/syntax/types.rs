//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor and no inherent impl. A kind is a
//! node of the standard's parsing results, a token of its tokenizer, or one of the few things the tree keeps that the standard
//! does not name; the element table holds its number.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

/// What an element of a CSS syntax tree is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    /// A node of CSS Syntax Level 3's parsing results.
    Node(Node),
    /// A token of CSS Syntax Level 3's tokenizer.
    Token(Token),
    /// What the tree keeps so that its tokens spell the file, which the standard does not name.
    Extra(Extra),
}

/// The role of an element under its parent: under a declaration, its name, each component value of its value, and the `important`
/// of its `!important`; every other edge has none. `syntax_child.field` names it.
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
    /// The name of a declaration, the property it sets: `color` of `color: red`.
    Name,
    /// A component value of a declaration's value: `red` of `color: red`.
    Value,
    /// The keyword of a declaration's `!important`.
    Important,
}

/// A word of a declaration: a part of it that is no white space and no comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Word<'a> {
    /// What it is.
    pub kind: Kind,
    /// The text of it as it is written; none for a node.
    pub spelled: &'a str,
    /// What the library decoded of it.
    pub(crate) decoded: Option<&'a str>,
}
