//! The types of the `programs` part: what the words over bodies, ranges and ordered tables are asked, as their steps state them.

use crate::domain::code::types::{MemoSite, WordFlag, WordSlot};
use crate::domain::encoding::types::ReadyBody;
use crate::domain::steps::types::{
    MadeTableId, MatchColumns, NearestEdges, RangeBound, ReadColumn, Register, StepBody, StepMemo,
    StepOperand, StepOperands, StepRows,
};
use ryo_values::domain::plan_orders::types::OrderBound;

/// How the answer of a step over an inline body is kept: the storage's code in the word's flag, the site, and the registers
/// that key it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeptAnswer {
    /// The storage, in the flag's `STORAGE` bits: 0 not kept, 1 once per file, 3 by the values of the registers.
    pub(crate) storage: WordFlag,
    /// The memo site, or `NO_SLOT` when the answer is not kept.
    pub(crate) site: MemoSite,
    /// The registers that key it.
    pub(crate) key: Vec<WordSlot>,
}

/// What a read of the nearest rows of an ordered table is asked.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NearestRows<'n> {
    /// The ordered table.
    pub(crate) table: MadeTableId,
    /// The key group's cells.
    pub(crate) key: &'n StepOperands,
    /// The bound.
    pub(crate) bound: &'n StepOperand,
    /// Which rows are nearest.
    pub(crate) side: OrderBound,
    /// Where each row's handle goes.
    pub(crate) out: Register,
}

/// What a read of a range of a bucket is asked.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RangeRead<'r> {
    /// Where the rows are read from.
    pub(crate) source: StepRows,
    /// The columns the bucket is matched on.
    pub(crate) columns: &'r MatchColumns,
    /// The bucket's values.
    pub(crate) values: &'r StepOperands,
    /// The column the range is on.
    pub(crate) range: ReadColumn,
    /// The low end, when there is one.
    pub(crate) low: Option<&'r RangeBound>,
    /// The high end, when there is one.
    pub(crate) high: Option<&'r RangeBound>,
    /// Where each row's handle goes.
    pub(crate) out: Register,
    /// Where the range column of each row goes, when it is kept.
    pub(crate) range_out: Option<Register>,
}

/// What a walk to the nearest element is asked.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NearestWalk<'n> {
    /// What is walked.
    pub(crate) edges: &'n NearestEdges,
    /// What the nearest element must hold.
    pub(crate) condition: &'n StepBody,
    /// The register the element under test is in.
    pub(crate) element: Register,
    /// The register holding the start.
    pub(crate) start: Register,
    /// Where the nearest element goes.
    pub(crate) out: Register,
    /// Whether the answer is kept.
    pub(crate) memo: &'n StepMemo,
}

/// The edges a nearest walk derives, made ready: their body and the register the element whose edges are asked is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DerivedEdges {
    /// The edges' body.
    pub(crate) edges: ReadyBody,
    /// The register the element whose edges are asked is in.
    pub(crate) from: Register,
}
