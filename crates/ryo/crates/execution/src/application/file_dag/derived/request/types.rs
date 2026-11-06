//! Counts of the plans a request has interned.

/// How many distinct plans a request has interned.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct PlanCount(pub usize);
