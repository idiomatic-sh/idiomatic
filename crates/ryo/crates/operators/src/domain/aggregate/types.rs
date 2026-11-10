//! Which reduction an aggregate formats.

/// Whether a group is formatted with its count alone or with the digest of its values too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Aggregation {
    /// The keys and the count.
    Counted,
    /// The keys, the digest and the count.
    Digested,
}
