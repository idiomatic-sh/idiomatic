//! Which reduction an aggregate formats.

/// Whether a group is formatted with its count alone or with the digest of its values too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Aggregation {
    /// The keys and the count.
    Counted,
    /// The keys, the digest and the count.
    Digested,
}

/// What a counted grouping lowers to: the columns that make a group and what is counted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GroupParts {
    /// The columns that make a group.
    pub(crate) keys: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    /// The column whose distinct values are counted in place of the rows.
    pub(crate) distinct: Option<ryo_values::domain::plan_parts::types::ColumnIndex>,
}
