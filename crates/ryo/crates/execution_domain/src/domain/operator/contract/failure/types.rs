//! The types of the `failure` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operating::errors::OperatingError;

/// Whether the latch has recorded a failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LatchState {
    /// A failure was recorded.
    Failed,
    /// No failure was recorded.
    Open,
}

/// The first failure an evaluation records, kept until it is taken.
#[derive(Default, Debug)]
pub struct FailureLatch(pub Option<OperatingError>);
