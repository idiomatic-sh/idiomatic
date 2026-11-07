//! The types of the `scopes` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether a function writes a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Namelessness {
    /// A `fn` that writes none: what the parser makes of a broken one.
    Nameless,
    /// Anything else.
    Named,
}
