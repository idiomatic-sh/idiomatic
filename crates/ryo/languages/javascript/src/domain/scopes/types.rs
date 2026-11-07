//! The types of the `scopes` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether a node opens a lexical environment of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    /// The node opens one: its names are not seen outside it.
    Opens,
    /// The node does not: its names belong to the scope around it.
    Continues,
}
