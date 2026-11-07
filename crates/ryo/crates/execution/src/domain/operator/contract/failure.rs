//! One first-error latch shared by borrowed semantic drivers.
use crate::domain::operating::errors::OperatingError;

/// Whether the latch has recorded a failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LatchState {
    Failed,
    Open,
}

#[derive(Default)]
pub(crate) struct FailureLatch(Option<OperatingError>);
impl FailureLatch {
    pub(crate) fn failed(&self) -> LatchState {
        if self.0.is_some() {
            LatchState::Failed
        } else {
            LatchState::Open
        }
    }
    /// Skip every callback after the first failure and preserve that original engine error.
    pub(crate) fn run<T>(
        &mut self,
        body: impl FnOnce() -> Result<T, OperatingError>,
    ) -> Result<T, ()> {
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
    fn record(&mut self, error: OperatingError) {
        if self.0.is_none() {
            self.0 = Some(error);
        }
    }
    pub(crate) fn take(&mut self) -> Option<OperatingError> {
        self.0.take()
    }
}
