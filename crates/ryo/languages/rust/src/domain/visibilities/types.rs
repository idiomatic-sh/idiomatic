//! The types of the `visibilities` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether a node is a declaration `defines` has a row for, known by the node itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Declaring {
    /// It is.
    Declares,
    /// It is not.
    Silent,
}
