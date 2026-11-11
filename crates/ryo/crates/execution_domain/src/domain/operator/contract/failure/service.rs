//! The `failure` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::types::{FailureLatch, LatchState};
use crate::domain::operating::errors::OperatingError;

/// Whether a failure was recorded.
pub fn failed(latch: &FailureLatch) -> LatchState {
    if latch.0.is_some() {
        LatchState::Failed
    } else {
        LatchState::Open
    }
}

/// Skip every callback after the first failure and preserve that original engine error.
///
/// # Errors
///
/// `Err(())` when a failure was already recorded, without calling `body`, and when `body` fails, after
/// recording its error for [`FailureLatch::take`].
pub fn run<T>(
    latch: &mut FailureLatch,
    body: impl FnOnce() -> Result<T, OperatingError>,
) -> Result<T, ()> {
    if latch.0.is_some() {
        return Err(());
    }
    match body() {
        Ok(value) => Ok(value),
        Err(error) => {
            record(latch, error);
            Err(())
        }
    }
}

pub(crate) fn record(latch: &mut FailureLatch, error: OperatingError) {
    if latch.0.is_none() {
        latch.0 = Some(error);
    }
}

/// The recorded failure, leaving the latch open.
pub fn take(latch: &mut FailureLatch) -> Option<OperatingError> {
    latch.0.take()
}
