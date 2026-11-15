//! Borrowed reduction contracts; no engine authority.
/// Key meaning of a prepared grouping order.
#[derive(Clone, Copy, Debug)]
pub enum GroupColumns<'a> {
    Whole,
    Columns(&'a [ryo_values::domain::plan_parts::types::ColumnIndex]),
}
/// Borrowed count/digest program, without scheduler or resource policy.
#[derive(Clone, Copy, Debug)]
pub enum AggregationPlan<'a> {
    Count {
        keys: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        distinct: Option<ryo_values::domain::plan_parts::types::ColumnIndex>,
    },
    Digest {
        keys: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        over: ryo_values::domain::plan_parts::types::ColumnIndex,
    },
}
/// Collected counts check the operand first; streaming counts check keys first.
#[derive(Clone, Copy, Debug)]
pub enum AggregationCheckOrder {
    Collected,
    Streaming,
}
/// Semantic group descriptors are written into engine-owned storage in first-row order.
pub trait GroupOutput {
    fn clear(&mut self);
    fn push(
        &mut self,
        first: ryo_values::domain::rows::types::RowIndex,
        counted: ryo_values::domain::rows::types::RowCount,
        digest: GroupDigest,
    );
    fn sort_first(&mut self);
}
/// An engine bridge resolves the value against its already acquired symbol/text backing.
pub trait DigestValues {
    fn datum(&self, value: ryo_relations::domain::relations::types::Value) -> DigestDatum<'_>;
}
/// All text is borrowed from engine-acquired backing, including authored literal lists.
#[derive(Clone, Copy, Debug)]
pub enum DigestDatum<'a> {
    Unlocated,
    Text(ryo_values::domain::row_texts::types::CellText<'a>),
    Number(ryo_values::domain::plan_parts::types::WholeNumber),
    Flag(ryo_values::domain::plan_parts::types::FlagState),
    Numbers(&'a [ryo_values::domain::plan_parts::types::WholeNumber]),
    Texts(&'a [ryo_values::domain::plan_parts::types::LiteralText]),
}
/// A value hash in a saturated original group coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AggregationHash {
    pub group: GroupCoordinate,
    pub hash: ValueHash,
}

/// A group's original coordinate, saturated to 32 bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupCoordinate(pub u32);

/// The folded digest a group finishes with; zero for a counted group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupDigest(pub i64);
/// Mutable reduction storage is borrowed from one engine invocation, never retained by semantics.
pub trait StreamingReduction {
    /// # Errors
    ///
    /// [`ContractError::Interrupted`] when an engine-retained error interrupts the grouping.
    fn group(
        &mut self,
        key: &[ryo_relations::domain::relations::types::Value],
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<ryo_values::domain::rows::types::RowIndex, ContractError>;
    fn insert_value(
        &mut self,
        group: ryo_values::domain::rows::types::RowIndex,
        value: ryo_relations::domain::relations::types::Value,
    ) -> FirstOccurrence;
    fn count(
        &mut self,
        group: ryo_values::domain::rows::types::RowIndex,
    ) -> Option<&mut ryo_values::domain::rows::types::RowCount>;
    fn datum(&self, value: ryo_relations::domain::relations::types::Value) -> DigestDatum<'_>;
    fn append_hash(&mut self, hash: AggregationHash);
}

use crate::domain::contract::errors::ContractError;
use crate::domain::contract::types::FirstOccurrence;

/// Whether two rows belong to the same group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroupMembership {
    /// The rows share their group key.
    Together,
    /// The rows differ in their group key.
    Apart,
}

/// The stable hash of one resolved value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValueHash(pub u64);

/// The engine-owned scratch the distinct value hashes of one group are collected in.
pub struct HashScratch<'a>(pub &'a mut Vec<u64>);

/// The hashes of a group's values a worker keeps between groups, for a group digest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct GroupHashes(pub(crate) Vec<u64>);

/// What a group digest folds a group's value hashes to: how many distinct values, and their digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FoldedDigest {
    /// How many distinct values.
    pub(crate) counted: ryo_values::domain::rows::types::RowCount,
    /// Their digest.
    pub(crate) digest: GroupDigest,
}

/// The row count each group of a streamed reduction reached, in group order.
#[derive(Clone, Copy, Debug)]
pub struct GroupCounts<'a>(pub &'a [usize]);

/// What a streamed reduction finishes into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finishing {
    /// Each group keeps its counted rows.
    Counted,
    /// Each group keeps its folded value hashes.
    Digested,
}
