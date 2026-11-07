//! The types of the `scopes` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether a kind opens a scope of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scopeness {
    /// A function declaration or a function literal.
    Scope,
    /// Anything else.
    Inner,
}
