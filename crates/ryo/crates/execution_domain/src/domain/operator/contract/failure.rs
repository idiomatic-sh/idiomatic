//! One first-error latch shared by borrowed semantic drivers.
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
pub struct FailureLatch(Option<OperatingError>);
impl FailureLatch {
    /// Whether a failure was recorded.
    pub fn failed(&self) -> LatchState {
        if self.0.is_some() {
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
    pub fn run<T>(&mut self, body: impl FnOnce() -> Result<T, OperatingError>) -> Result<T, ()> {
        if self.0.is_some() {
            return Err(());
        }
        match body() {
            Ok(value) => Ok(value),
            Err(error) => {
                self.record(error);
                Err(())
            }
        }
    }
    pub(crate) fn record(&mut self, error: OperatingError) {
        if self.0.is_none() {
            self.0 = Some(error);
        }
    }
    /// The recorded failure, leaving the latch open.
    pub fn take(&mut self) -> Option<OperatingError> {
        self.0.take()
    }
}
