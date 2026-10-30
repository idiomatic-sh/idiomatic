//! The types of the `references` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages;

/// What a call calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Called {
    /// The element that writes the callee: the path of a call, the name of a method call.
    pub callee: languages::Position,
    /// The name it is called by.
    pub target: languages::Symbol,
}

/// Whether a path writes exactly one name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Singleness {
    /// It writes one.
    Single,
    /// It writes none, or several.
    Several,
}

/// Whether a path is a use of a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Usage {
    /// The outermost path of a use of a name.
    Written,
    /// Part of a longer path, or the path of a `use`.
    Skipped,
}
