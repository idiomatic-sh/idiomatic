//! The types of the `pruning` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether one of a plan's roots reads a step, itself or through the steps it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StepRead {
    /// A root reads it.
    Read,
    /// No root reads it.
    Unread,
}
