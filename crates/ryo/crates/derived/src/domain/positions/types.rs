//! The types of the `positions` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Borrowed row positions: the bare positions inside are named here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowSlots<'a>(pub &'a [u32]);

/// Borrowed membership marks over a dense element space: the bare marks inside are named here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseMarks<'a>(pub &'a [bool]);
