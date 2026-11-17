//! The work of one worker's graph walks: what a walk met and has left to expand, and what each walked start reached.

use crate::domain::physical_plan::types::keyed_rows::value::{IndexKeyHash, IndexedRow};
use crate::domain::physical_plan::types::visited_set::value::VisitedSet;
use ryo_relations::domain::relations::types::Value;

/// The work of the graph walks: what one walk met and has left to expand, and, for a seeded closure, what each walked start
/// reached, so a start is walked once per file however many rows bind it.
#[derive(Debug, Default)]
pub(crate) struct WalkScratch {
    /// The entities the walk from one start has met.
    pub(crate) visited: VisitedSet,
    /// The edge rows whose destinations it has left to expand.
    pub(crate) frontier: Vec<ryo_values::domain::rows::types::RowIndex>,
    /// What each walked start reached, ascending, run after run.
    pub(crate) reached: Vec<Value>,
    /// Each walked start's run of `reached`, by the hash of its partition and source.
    pub(crate) runs: hashbrown::HashTable<ReachedRun>,
    /// One edge row of each start, in the order the walks start from them.
    pub(crate) starts: Vec<IndexedRow>,
    /// The key columns a walk reads: of the rows it starts from, of the edges it keys, and of an edge's far end.
    pub(crate) columns: WalkColumns,
    /// The rows a seeded closure reached, each as the join's logical right row, for the join's probe; an axis's verdict
    /// on one domain element.
    pub(crate) rows: ryo_values::domain::rows::types::Table,
    /// The entities of a breadth-first walk's level, or the cells of an axis's domain element.
    pub(crate) entities: Vec<Value>,
    /// The entities of the next level of a breadth-first walk.
    pub(crate) next: Vec<Value>,
    /// The row an axis's domain filter writes.
    pub(crate) written: Vec<Value>,
}

/// The key columns a walk reads, kept with their capacity between nodes.
#[derive(Debug, Default)]
pub(crate) struct WalkColumns {
    /// The columns of a start row that bind a start: a left row's for a seeded closure.
    pub(crate) seed: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    /// The columns an edge is keyed by: its partition, then its near end.
    pub(crate) near: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    /// The columns of an edge's partition and its far end, which key the edges walked next.
    pub(crate) far: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
}

/// One walked start's place in `WalkScratch::reached`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReachedRun {
    /// The hash of the start's partition and source.
    pub(crate) hash: IndexKeyHash,
    /// The left row that bound it first, whose cells hold its partition and source.
    pub(crate) first: IndexedRow,
    /// Where its run starts in `reached`.
    pub(crate) start: ReachedPlace,
    /// How many entities it reached.
    pub(crate) count: ReachedCount,
}

/// A place in `WalkScratch::reached`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReachedPlace(pub(crate) u32);

/// How many entities one walked start reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReachedCount(pub(crate) u32);
