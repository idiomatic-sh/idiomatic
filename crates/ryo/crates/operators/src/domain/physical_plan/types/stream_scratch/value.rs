//! The work of the chains of streamed stages one worker runs: each chain's nodes and each stage's chunk and own state, kept
//! with their capacity across nodes and files, no chunk keeping more room than the bound.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::keyed_rows::value::KeyedRows;
use crate::domain::physical_plan::types::{IndexPlace, InputRows, PlanNodeId};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// The cells a stage's chunk holds before its reader reads it, checked between input rows: 16 Ki cells (256 KiB of
/// 16-byte cells). A settled stage keeps no more room than this in its tables.
pub(crate) const STREAM_CHUNK_CELLS: usize = 1 << 14;

/// The cells a stage writes before it hands its rows on, checked between its input rows: a chain's stages are bound by
/// `STREAM_CHUNK_CELLS`, its tail by none; a held kernel that runs the same rows (a seeded closure's join) is bound by none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CellBound(pub(crate) usize);

/// The rows a stage reads in one call: those of `rows` from row `from`, in order, read in place whatever they are (a
/// relation's columns, the tree's rows, a chunk or a held table), never copied for the call.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RowChunk<'c> {
    /// The rows: a chain's head chunk or the stage before's chunk, or a held kernel's input (a seeded closure's left).
    pub(crate) rows: InputRows<'c>,
    /// The first row not read yet.
    pub(crate) from: RowIndex,
}

/// How far a stage read its rows in one call: how many it read, and why it stopped.
#[derive(Debug, PartialEq)]
pub(crate) struct ChunkRead {
    /// The rows it read, the faulting row included.
    pub(crate) read: RowCount,
    /// Why it stopped.
    pub(crate) stop: ChunkStop,
}

/// Why a stage stopped reading its rows.
#[derive(Debug, PartialEq)]
pub(crate) enum ChunkStop {
    /// It read every row.
    Ended,
    /// Its rows reached the bound: its reader reads them before it reads on, from the row after the last it read.
    Full,
    /// A row's fault: it keeps the rows before it and reads no more.
    Faulted(PortableOperatorFault),
    /// A hard fault, the chain's at this stage: it reads no more.
    Hard(PortableOperatorFault),
}

/// The work of the chains of streamed stages a worker runs: one chain's scratch for each chain running at once (a chain
/// that readies a held source runs that source's chain inside it), kept with its capacity across nodes and files.
#[derive(Debug, Default)]
pub(crate) struct StreamScratch {
    /// The scratch of each chain not running now, taken by the next chain to run and given back when it is done.
    pub(crate) levels: Vec<ChainScratch>,
}

/// One chain's scratch: its nodes, head first, a head row, the head's chunk, and each stage's state.
#[derive(Debug, Default)]
pub(crate) struct ChainScratch {
    /// The chain's nodes, from the head stage to the tail.
    pub(crate) chain: Vec<PlanNodeId>,
    /// A head row, decoded from a relation's columns or the tree.
    pub(crate) row: Vec<Value>,
    /// The head's rows decoded for one call of the head stage, no more cells than the bound.
    pub(crate) head: Table,
    /// Each stage's state, by its place in the chain.
    pub(crate) stages: Vec<StageScratch>,
}

/// One stage's state for one run of its chain: the chunk of rows it wrote that its reader has not read, what it read,
/// whether it stopped at a fault, and what it keeps for itself across its input rows.
#[derive(Debug, Default)]
pub(crate) struct StageScratch {
    /// The rows it wrote that its reader has not read yet: its chunk. The tail writes its node's table instead.
    pub(crate) out: Table,
    /// How many input rows it read.
    pub(crate) read: RowCount,
    /// The fault it stopped at; it reads no row after it.
    pub(crate) fault: Option<PortableOperatorFault>,
    /// What the stage keeps for itself across its input rows, its own beside every other stage's of the chain.
    pub(crate) own: StageOwn,
}

/// What one stage keeps for itself across its input rows and chunks: an extension's values, a tail's repeats, and a probe's
/// right side (its own index, the right rows its filter kept and its whole-row key columns), none shared with another stage.
#[derive(Debug, Default)]
pub(crate) struct StageOwn {
    /// An extension's values, resolved once before its first row.
    pub(crate) values: Vec<Value>,
    /// The rows a tail that drops repeated rows kept so far, keyed by their cells in its table.
    pub(crate) repeats: KeyedRows,
    /// A probe's own index of its right rows; a probe of a shared index or of the tree builds none.
    pub(crate) index: KeyedRows,
    /// The right rows a probe's filter kept, read by every left row.
    pub(crate) kept: Table,
    /// A probe's key columns of a whole-row key: its left's, then its right's.
    pub(crate) whole: [Vec<ColumnIndex>; 2],
    /// Whether a probe did its work before its first left row.
    pub(crate) work: ProbeWork,
    /// A probe's right rows, by their place among its chain's right sides.
    pub(crate) right: RightPlace,
    /// The rows a probe was fed from its right side: its right rows, or the tree rows its walks found.
    pub(crate) fed: RowCount,
    /// The shared index a probe opened for its chain's run, closed after it.
    pub(crate) shared: Option<IndexPlace>,
}

/// Whether a probe did its work before its first left row: its right filter, its index build, its extremes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ProbeWork {
    /// No left row reached it yet.
    #[default]
    Pending,
    /// Its first left row did it.
    Done,
}

/// The work a worker's chains did: how many times they called a stage (each call a run of rows), and the most rows they
/// handed from a stage to its reader in one chunk.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ChainWork {
    /// The stage calls.
    pub(crate) calls: StageCalls,
    /// The most rows of one chunk handed on.
    pub(crate) handed: RowCount,
}

/// How many times a worker's chains called a stage.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StageCalls(pub(crate) usize);

/// A stage's place in its chain, the head stage's first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StagePlace(pub(crate) usize);

/// A probe's right side, by its place among its chain's right sides, in chain order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RightPlace(pub(crate) usize);

impl crate::domain::kept_room::types::KeptRoom for StreamScratch {
    fn kept(&mut self) {
        for stage in self.levels.iter_mut().flat_map(|level| &mut level.stages) {
            stage.own.repeats.kept();
            stage.own.index.kept();
        }
    }

    #[cfg(test)]
    fn room(&self) -> crate::domain::kept_room::types::Room {
        self.levels
            .iter()
            .flat_map(|level| &level.stages)
            .map(|stage| stage.own.repeats.room().max(stage.own.index.room()))
            .max()
            .unwrap_or_default()
    }
}

impl ChainScratch {
    /// Empties the chain after its run: its head chunk and each stage, none keeping more room in its tables than the bound.
    pub(crate) fn settled(&mut self) {
        self.head.kinds.clear();
        self.head.cells.clear();
        self.head.count = RowCount(0);
        // A chunk holds no more cells than the bound, but its room grows past it as it is decoded row by row.
        if self.head.cells.capacity() > STREAM_CHUNK_CELLS {
            self.head.cells.shrink_to(STREAM_CHUNK_CELLS);
        }
        for stage in &mut self.stages {
            stage.settled();
        }
    }
}

impl StageScratch {
    /// Empties the stage after its chain's run, keeping no more room in its tables than the bound, and its index empty.
    pub(crate) fn settled(&mut self) {
        for table in [&mut self.out, &mut self.own.kept] {
            table.kinds.clear();
            table.cells.clear();
            table.count = RowCount(0);
            if table.cells.capacity() > STREAM_CHUNK_CELLS {
                table.cells.shrink_to(STREAM_CHUNK_CELLS);
            }
        }
        self.own.index.cleared();
        self.fault = None;
    }
}
