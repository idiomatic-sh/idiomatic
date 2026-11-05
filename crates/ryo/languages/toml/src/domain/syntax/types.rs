//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. A kind of node or token is a node, a token or an extra, named in kebab case (`keyval`,
//! `basic-string`, `ws`); the element table holds its number.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::TomlNode;
use crate::domain::tokens::types::Token;

/// What a node or a token of a TOML syntax tree is: a node or a token of `toml.abnf`, or an extra the standard lacks. The element table
/// holds its number: the nodes first, then the tokens, then the extras, each in the order of its enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    /// A node.
    Node(TomlNode),
    /// A token.
    Token(Token),
    /// A kind the standard lacks.
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

/// Text of the file, as it is written: what an element spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Text<'a>(pub &'a str);

/// The role of an element under its parent, named by the element of `toml.abnf` it is in its parent's rule: a key-value's `key` and
/// `val` (`keyval = key keyval-sep val`), a table header's `key` (`std-table = std-table-open key std-table-close`, `array-table` the
/// same), each `simple-key` of a key (`dotted-key = simple-key 1*( dot-sep simple-key )`), and each `val` of an array
/// (`array-values`). Every other edge has none: punctuation and white space, which their kinds name, and the key-values of a table or an
/// inline table, which are the whole of what their rule repeats. `syntax_child.field` names it.
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
    /// The key of a key-value or of a table header: `a.b` of `a.b = 1` and of `[a.b]`.
    Key,
    /// The value of a key-value, or an element of an array: `1` of `a = 1`, each of `[1, 2]`.
    Val,
    /// One part of a key: `a` and `b` of `a.b`.
    SimpleKey,
}
