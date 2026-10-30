//! The types of the `names` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages::Position;

/// Whether an occurrence of a name is read where it occurs, or binds or keys it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reference {
    /// The name is read where it occurs: a reference.
    Use,
    /// The name is declared, or used as a key, where it occurs.
    Binding,
}

/// A name and the string literal it is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Assignment {
    /// The name being bound.
    pub binding: Position,
    /// The string literal it is bound to.
    pub value: Position,
}
