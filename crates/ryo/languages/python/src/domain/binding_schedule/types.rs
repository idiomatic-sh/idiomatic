//! The values of the binding schedule.

/// Index of the next deferred binding phase to drain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PhaseCursor(pub usize);
