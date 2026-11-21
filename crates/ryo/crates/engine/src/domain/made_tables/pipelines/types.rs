//! The types of the `pipelines` part: the batches a pipeline runs over and the room they lie in, and the kinds of its
//! stages.

use crate::domain::code::types::PoolAt;
use crate::domain::indexes::types::{IndexArena, RowRun};
use crate::domain::stages::types::{BatchRow, FanRows, RowKept};
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// One batch of a pipeline at one depth: a column for each register, how many rows they hold, and what the stage that
/// gives several rows for one wrote for the next depth.
#[derive(Debug, Default)]
pub(crate) struct BatchLevel {
    /// The columns, by register.
    pub(crate) columns: Vec<Vec<CellWord>>,
    /// How many rows the batch holds.
    pub(crate) rows: Count,
    /// What a stage that gives several rows for one wrote.
    pub(crate) fan: FanRows,
    /// The rows a stage that keeps some rows kept.
    pub(crate) keep: Vec<BatchRow>,
    /// The rows a `Use` stage's patterns yielded for the batch, row by row.
    pub(crate) answers: Vec<CellWord>,
    /// For each row of the batch a `Use` stage reads, the rows of `answers` its key holds.
    pub(crate) spans: Vec<RowRun>,
    /// For each row of the batch a `Use` stage reads, whether its key's pattern has a row.
    pub(crate) found: Vec<RowKept>,
}

/// The room one pipeline runs in: its batches by depth, which registers each stage must carry on, and the scratch of a
/// key, of a set and of a `Use` stage's answers. Kept between pipelines, never freed.
#[derive(Debug, Default)]
pub(crate) struct PipelineRoom {
    /// The batches, by depth: a stage that gives several rows for one fills the next.
    pub(crate) levels: Vec<BatchLevel>,
    /// For each stage, the registers written at or before it and read after it, one stage after the other.
    pub(crate) live: Vec<Register>,
    /// Where each stage's registers start in `live`; one more entry than stages.
    pub(crate) starts: Vec<Count>,
    /// The cells of a key.
    pub(crate) key: Vec<CellWord>,
    /// The cells of a set a `Keep` tests against.
    pub(crate) set: Vec<CellWord>,
    /// The index that groups a batch by a `Use` stage's key.
    pub(crate) grouping: IndexArena,
    /// What each stage of the pipeline reads and writes.
    pub(crate) touched: Vec<StageRegisters>,
}

/// The registers one stage reads and the registers it writes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StageRegisters {
    /// The registers it reads.
    pub(crate) read: Vec<Register>,
    /// The registers it writes.
    pub(crate) written: Vec<Register>,
}

/// The kinds of stage a pipeline holds, and its write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StageKind {
    /// `Scan`.
    Scan,
    /// `Probe`.
    Probe,
    /// `Product`.
    Product,
    /// `Use` of a pattern in call form.
    Use,
    /// `Latest`.
    Latest,
    /// `Read`.
    Read,
    /// `Keep`.
    Keep,
    /// `Call`.
    Call,
    /// `Render`.
    Render,
    /// `Walk`.
    Walk,
    /// The write of the pipeline's row.
    Write,
}

/// A run of operands in the pool: where the first is, and how many.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OperandRun {
    /// Where the first operand is.
    pub(crate) first: PoolAt,
    /// How many operands.
    pub(crate) count: Count,
}

/// The kinds of stage that give several rows for one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FanKind {
    /// `Probe` in rows mode.
    Probe,
    /// `Product`.
    Product,
    /// `Latest`.
    Latest,
    /// `Walk`.
    Walk,
    /// `Use` in rows mode, its patterns asked.
    Use,
}
