//! The types of the `deferring` part.

/// How many rows an access gives for one row it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AccessRows {
    /// One or none: a read, a test, a decision. A read may wait past it.
    AtMostOne,
    /// Any number: a read that waited past it would be made once for each.
    Several,
}
