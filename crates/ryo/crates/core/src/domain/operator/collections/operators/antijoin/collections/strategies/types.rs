//! The types of the `strategies` collection: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether no row on the right of an antijoin matches a row on the left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Absence {
    /// No row of the right matches.
    Absent,
    /// A row of the right matches.
    Present,
}
