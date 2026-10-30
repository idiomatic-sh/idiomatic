//! The types of the `gomod` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::modules::types::Indirectness;

/// A word of a directive: a verb, an argument, a parenthesis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Word(pub String);

/// A line of a manifest, from one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LineNumber(pub usize);

/// The text of one line of a manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LineText<'a>(pub &'a str);

/// One line of a manifest, read: its words, and whether its comment says `indirect`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Line {
    /// The words.
    pub words: Vec<Word>,
    /// What its comment says of a requirement.
    pub indirect: Indirectness,
}

/// A block that is open: the directive it belongs to, and the line it opened at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Block {
    /// The directive.
    pub verb: Word,
    /// The line.
    pub line: LineNumber,
}

/// Which directive a verb is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Directive {
    /// `module`.
    Module,
    /// `require`.
    Require,
    /// `replace`.
    Replace,
    /// `use`, of a workspace.
    Use,
    /// Any other verb, which declares nothing here.
    Other,
}
