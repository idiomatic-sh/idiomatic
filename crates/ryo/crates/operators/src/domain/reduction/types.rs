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
        digest: i64,
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
    Text(&'a str),
    Number(ryo_values::domain::plan_parts::types::WholeNumber),
    Flag(ryo_values::domain::plan_parts::types::FlagState),
    Numbers(&'a [ryo_values::domain::plan_parts::types::WholeNumber]),
    Texts(&'a [ryo_values::domain::plan_parts::types::LiteralText]),
}
/// A value hash in a saturated original group coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AggregationHash {
    pub group: u32,
    pub hash: u64,
}
/// Mutable reduction storage is borrowed from one engine invocation, never retained by semantics.
pub trait StreamingReduction {
    fn group(
        &mut self,
        key: &[ryo_relations::domain::relations::types::Value],
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<ryo_values::domain::rows::types::RowIndex, SemanticInterrupted>;
    fn insert_value(
        &mut self,
        group: ryo_values::domain::rows::types::RowIndex,
        value: ryo_relations::domain::relations::types::Value,
    ) -> bool;
    fn count(&mut self, group: ryo_values::domain::rows::types::RowIndex) -> Option<&mut usize>;
    fn datum(&self, value: ryo_relations::domain::relations::types::Value) -> DigestDatum<'_>;
    fn append_hash(&mut self, hash: AggregationHash);
}

use crate::domain::contract::types::SemanticInterrupted;
