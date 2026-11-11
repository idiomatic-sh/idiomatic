//! One request-bound graph plan with invocation-local traversal buffers. Exact cache identity
//! and bounded retention stay in `CacheBackend` and the graph tables of the owner.

/// How many leading request columns partition a reach demand.
#[derive(Clone, Copy, Debug)]
pub(super) struct PartitionWidth(pub(super) usize);

/// A position within the left key, by which one request column is bound.
#[derive(Clone, Copy, Debug)]
pub(super) struct KeyPosition(pub(super) usize);
