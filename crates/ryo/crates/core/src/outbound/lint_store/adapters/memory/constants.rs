//! The constants of the memory store adapter.

/// How many stores the adapter keeps: the last run's, which the next run's replaces.
#[cfg(test)]
pub(super) const KEPT_STORE: usize = 1;
