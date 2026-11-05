//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. A kind of node or token is a node, a token or an extra, named in kebab case; the element table holds its number.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// What a node or a token of a json syntax tree is: a node or a token of the standard, or an extra the standard lacks. The element table
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

/// The name being made, written to as text: a number is formatted straight into it, with nothing allocated for the file.
pub(crate) struct NameWriter<'a>(pub &'a mut languages::Symbols);

impl std::fmt::Write for NameWriter<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        languages::push_text(self.0, languages::NameText(s));
        Ok(())
    }
}

/// Text of the file, as it is written: what an element spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Text<'a>(pub &'a str);

/// Whether a string is the name of a member, a key, or a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Keyness {
    /// The name of a member.
    Key,
    /// A value, or no string.
    Other,
}

/// How the text of a file is read: the grammar of the standard alone, or the grammar of JSON with comments, which also accepts
/// comments and a comma after the last member or value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dialect {
    /// JSON as RFC 8259 defines it.
    Strict,
    /// JSON with comments, as the configuration files of TypeScript and VS Code are written.
    Jsonc,
}

/// Whether an element is a value of the grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Valueness {
    /// A string, a number, a literal, an object or an array.
    Value,
    /// Anything else.
    Other,
}

/// The role of an element under its parent, named by the part of RFC 8259's grammar it is: a member's `name` and `value`
/// (`member = string name-separator value`, ECMA-404's name and value), each `value` of an array
/// (`array = begin-array [ value *( value-separator value ) ] end-array`) and the `value` of the document (`JSON-text = ws value ws`).
/// Every other edge has none: punctuation, white space and comments, which their kinds name, and the members of an object, the whole
/// of what its rule repeats. `syntax_child.field` names it.
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
    /// The name of a member: `"a"` of `"a": 1`.
    Name,
    /// The value of a member, an element of an array, or the document's value: `1` of `"a": 1`, each of `[1, 2]`.
    Value,
}
