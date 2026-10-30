//! The types of the `inlining` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

/// Whether an operation reads a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StepReading {
    /// One of its inputs is that step.
    Reads,
    /// None of its inputs is.
    Ignores,
}
