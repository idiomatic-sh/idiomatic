//! The types of the `table_groups` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use crate::domain::table_model::types::KeyIndex;

use ryo_relations::domain::relations::types::Value;

/// The digest of the set of distinct values a group holds; zero when none was asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SetDigest(pub i64);

/// A run of rows that agree on their keys, counted: the first of its rows, and how many it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Group {
    /// The first row of the group, in the order the rows came.
    pub first: RowIndex,
    /// What was counted for it.
    pub counted: RowCount,
    /// A digest of the set of distinct values it holds, when a digest was asked for; zero otherwise.
    pub digest: SetDigest,
}

/// The hashes of the values of a group.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Hashes(pub Vec<u64>);

/// The hash of the text or the number of a value, for a digest.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ValueHash(pub u64);

/// The room a count works in.
#[derive(Debug)]
pub(crate) struct CountRoom<'a> {
    /// The rows in the order of their keys.
    pub order: &'a mut KeyIndex,
    /// The groups found.
    pub groups: &'a mut Vec<Group>,
    /// The values of a group, to count the distinct ones.
    pub values: &'a mut Vec<Value>,
}

use ryo_values::domain::rows::types::{RowCount, RowIndex};
