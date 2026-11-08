//! Counts of the plans a request has interned.

/// How many distinct plans a request has interned.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlanCount(pub usize);

/// The positions of the request's inputs one derived view reads: the bare positions inside are named here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InputPositions<'a>(pub &'a [usize]);
