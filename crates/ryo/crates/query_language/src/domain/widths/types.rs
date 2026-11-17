//! The types of the `widths` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// How many columns the rows of an input have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InputWidth(pub u32);
