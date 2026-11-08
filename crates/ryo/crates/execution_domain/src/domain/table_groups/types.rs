//! The types of the `table_groups` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use crate::domain::table_texts::{
    service::datum,
    types::{Datum, Texts},
};
use ryo_operators::domain::reduction::types::{DigestDatum, DigestValues, GroupDigest, GroupOutput};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::row_texts::types::CellText;

/// The digest of the set of distinct values a group holds; zero when none was asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SetDigest(pub i64);

/// A run of rows that agree on their keys, counted: the first of its rows, and how many it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    /// The first row of the group, in the order the rows came.
    pub first: RowIndex,
    /// What was counted for it.
    pub counted: RowCount,
    /// A digest of the set of distinct values it holds, when a digest was asked for; zero otherwise.
    pub(crate) digest: SetDigest,
}

/// The hashes of the values of a group.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Hashes(pub Vec<u64>);

/// The room a count works in.
#[derive(Debug)]
pub struct CountRoom<'a> {
    /// The rows in the order of their keys.
    pub order: &'a [RowIndex],
    /// The groups found.
    pub groups: &'a mut Vec<Group>,
    /// The values of a group, to count the distinct ones.
    pub values: &'a mut Vec<Value>,
}

use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// The groups a reduction writes into, as its output port.
pub(crate) struct GroupTarget<'a>(pub &'a mut Vec<Group>);
impl GroupOutput for GroupTarget<'_> {
    fn clear(&mut self) {
        self.0.clear();
    }
    fn push(&mut self, first: RowIndex, counted: RowCount, digest: GroupDigest) {
        self.0.push(Group {
            first,
            counted,
            digest: SetDigest(digest.0),
        });
    }
    fn sort_first(&mut self) {
        self.0.sort_unstable_by_key(|group| group.first);
    }
}

/// The values of a digest, resolved through the texts of the table.
pub(crate) struct ResolvedValues<'a, 'text>(pub &'a Texts<'text>);
impl DigestValues for ResolvedValues<'_, '_> {
    fn datum(&self, value: Value) -> DigestDatum<'_> {
        match datum(value, self.0) {
            Datum::Unlocated => DigestDatum::Unlocated,
            Datum::Text(text) => DigestDatum::Text(CellText(text.0)),
            Datum::Number(number) => DigestDatum::Number(number),
            Datum::Flag(flag) => DigestDatum::Flag(flag),
            Datum::Numbers(numbers) => DigestDatum::Numbers(numbers),
            Datum::Texts(items) => DigestDatum::Texts(items),
        }
    }
}
