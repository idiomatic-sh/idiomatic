//! The types of the `digest` port: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::violations::types::Fingerprint;

/// The fingerprint of each line of a file, the first line first; a file ending in a newline has one more, empty,
/// line after it, so a whole-file violation anchored at line one always has one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineFingerprints(pub Vec<Fingerprint>);

/// The byte strings a digest is asked of, in order, each closed by a zero byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DigestParts<'a>(pub &'a [&'a [u8]]);

/// One line of a file, as the item fingerprint reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineText<'a>(pub &'a str);

/// How many whitespace characters a line starts with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Indent(pub usize);

/// A word of an item's declaration: its keyword, or the words that name it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemWord(pub Box<str>);

/// An item's declaration: its keyword and the words naming it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    /// `fn`, `impl`, `class` and kin.
    pub keyword: ItemWord,
    /// What it is named.
    pub name: ItemWord,
}

/// What a line declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Declares {
    /// An item.
    Item(Declaration),
    /// Nothing.
    Nothing,
}

/// A line whose indentation scopes the lines below it, and what it declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Open {
    /// Its indentation.
    pub indent: Indent,
    /// What it declares.
    pub declares: Declares,
    /// Whether its line ends where a body opens, so the lines after it belong to it even when they are not indented.
    pub body: Body,
}

/// Whether a line ends where a body opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    /// It ends with an opening brace.
    Opens,
    /// It does not.
    Absent,
}

/// Whether a line continues a declaration begun above or begins one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// It continues one: a closing bracket, an opening brace or a `where` clause.
    Continues,
    /// It begins a line of its own.
    Begins,
}

/// The first word of a line's text and what follows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Words<'a> {
    /// The word.
    pub word: LineText<'a>,
    /// What follows it, its whitespace included.
    pub rest: LineText<'a>,
}

/// Whether a line's qualifiers bind a name, which declares a function when it is bound to one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    /// A binding word came before the name.
    Bound,
    /// None did.
    Unbound,
}
