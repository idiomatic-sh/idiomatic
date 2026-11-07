//! The types of the `documents` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether an element can be a part of a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Keyness {
    /// It can: a bare key or a one-line quoted one.
    Part,
    /// It cannot.
    Other,
}
