//! The types of the `pipelines` part: the batches a pipeline runs over and the room they lie in, and the kinds of its
//! stages.

use crate::domain::code::types::PoolAt;
use crate::domain::indexes::types::{IndexArena, RowRun};
use crate::domain::stages::types::{BatchRow, CallScratch, FanRows, RowKept};
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_operators::domain::table_scratch::types::Patterns;
use ryo_relations::domain::relations::types::Count;

/// One batch of a pipeline at one depth: a column for each register, how many rows they hold, and what the stage that
/// gives several rows for one wrote for the next depth.
#[derive(Debug, Default)]
pub(crate) struct BatchLevel {
    /// The columns of the registers the running pipeline wrote at this depth.
    pub(crate) columns: LevelColumns,
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

/// The columns of one batch: a column for each register the RUNNING pipeline wrote at this depth, and no more. A pack
/// has a register for every value of every pipeline (the whole Python pack: over a hundred thousand), and a pipeline
/// writes a handful: kept by register, a worker held a list for each register of the pack at every depth of every
/// room, each at the size of the largest batch that register ever held (half of the run's memory). Kept by the place
/// a register was first written at in the run, the lists are as many as the widest pipeline has registers, and are
/// used again by the next pipeline.
#[derive(Debug, Default)]
pub(crate) struct LevelColumns {
    /// For each register of the pack, the place of its column in `held` and one more; nought: no column in this run.
    pub(crate) places: Vec<u32>,
    /// The columns, in the order their registers were first written in the run; those past `used` are spare.
    pub(crate) held: Vec<Vec<CellWord>>,
    /// The registers that have a column in this run: what a new run unsets.
    pub(crate) written: Vec<Register>,
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
    /// The regular expressions a `Keep` or a `Call` compiled, by source: kept between files.
    pub(crate) compiled: Patterns,
    /// The scratch a `Call`'s meanings work in.
    pub(crate) calls: CallScratch,
    /// The registers of the running pipeline that hold the file, by place: those a `Read` filled from a made table's
    /// own file column or from the node table's.
    pub(crate) files: Vec<Count>,
}

/// What was found of each pipeline the first time it ran, kept for the worker's life (a store serves one rule set, whose
/// code does not change): the registers each of its stages carries on, and that every stage of it is one the fill side
/// runs. A pipeline runs once a file for each table it fills, most of them over a handful of rows: found again for
/// every run, the pass over its stages cost more than its rows.
#[derive(Debug, Default)]
pub(crate) struct PipeMemo {
    /// Where each pipeline's registers lie in `live` and its starts in `starts`, by the pipeline's entry.
    pub(crate) by_entry: hashbrown::HashMap<u32, PipeAt>,
    /// The registers carried on, one pipeline after the other.
    pub(crate) live: Vec<Register>,
    /// Where each stage's registers start among its pipeline's, one pipeline after the other.
    pub(crate) starts: Vec<Count>,
}

/// One pipeline's stretches of a `PipeMemo`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PipeAt {
    /// Its registers: from this place of `live`, this many.
    pub(crate) live: (Count, Count),
    /// Its starts: from this place of `starts`, this many.
    pub(crate) starts: (Count, Count),
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
    /// `Call`, its meaning asked.
    Call,
}
