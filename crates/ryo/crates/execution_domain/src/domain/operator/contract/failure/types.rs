//! The types of the `failure` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operating::errors::OperatingError;

/// Whether the latch has recorded a failure.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::failure::types::LatchState;
/// fn named(_: Option<&LatchState>) {}
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LatchState {
    /// A failure was recorded.
    Failed,
    /// No failure was recorded.
    Open,
}

/// The first failure an evaluation records, kept until it is taken.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::failure::types::FailureLatch;
/// fn named(_: Option<&FailureLatch>) {}
/// ```
#[derive(Default, Debug)]
pub struct FailureLatch(pub Option<OperatingError>);
