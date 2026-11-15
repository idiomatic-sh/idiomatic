//! Which reduction an aggregate formats.

/// Whether a group is formatted with its count alone or with the digest of its values too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Aggregation {
    /// The keys and the count.
    Counted,
    /// The keys, the digest and the count.
    Digested,
}

/// What a grouping lowers to: the columns that make a group and what each group answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GroupParts {
    /// The columns that make a group.
    pub(crate) keys: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    /// What each group answers besides its keys.
    pub(crate) measure: GroupMeasure,
}

/// What a group answers besides its keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroupMeasure {
    /// Its row count: `group-count`.
    Rows,
    /// The count of the distinct values of a column: `group-count` with `distinct`.
    DistinctValues(ryo_values::domain::plan_parts::types::ColumnIndex),
    /// The digest of the set of distinct values of a column, and how many there are: `group-digest`.
    Digest(ryo_values::domain::plan_parts::types::ColumnIndex),
}
