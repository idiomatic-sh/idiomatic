//! One worker's node buffers: the rows of each materialized node of the open file, kept until its last consumer is done, and
//! their capacity kept across files.

use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::{NodeReaders, NodeRows, PlanNodeId};
use crate::domain::table_scratch::types::{LazyScratch, Memo, SelectScratch};
use ryo_relations::domain::relations::types::Value;

/// The buffers of one worker, reused across files.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
/// use ryo_operators::domain::physical_plan::types::{NodeRows, PlanNodeId};
/// let buffers = FileBuffers::default();
/// assert_eq!(buffers.rows(PlanNodeId(0)), NodeRows::Absent);
/// ```
#[derive(Debug, Default)]
pub struct FileBuffers {
    /// The rows of each node, by node id; a node not run or released holds none.
    pub(crate) nodes: Vec<Option<ryo_values::domain::rows::types::Table>>,
    /// The fault each node raised after its rows, by node id: its readers raise it once they have read every row before it.
    pub(crate) pending: Vec<Option<PortableOperatorFault>>,
    /// How many of each node's readers have not read it yet in this file, by node id.
    pub(crate) remaining: Vec<NodeReaders>,
    /// Each filtering node's comparison scratch, by node id, kept across files.
    pub(crate) decisions: Vec<NodeDecisions>,
    /// The one keyed-row table of the shared indexes, its capacity kept across files.
    pub(crate) index: KeyedRows,
    /// The row a kernel is reading, decoded from the facts one row at a time.
    pub(crate) row: Vec<Value>,
    /// The row a kernel is writing.
    pub(crate) written: Vec<Value>,
    /// The values a kernel collects: an extension's resolved values, a group's distinct values.
    pub(crate) values: Vec<Value>,
    /// The hashes of a group's values, which a group digest folds.
    pub(crate) hashes: crate::domain::reduction::types::GroupHashes,
    /// The place of each row `distinct` kept, by the hash of its cells, its capacity kept across nodes.
    pub(crate) seen: hashbrown::HashTable<ryo_values::domain::rows::types::RowCount>,
    /// The second keyed-row table a kernel reads beside `index`: an axis's domain. The same table, a second instance.
    pub(crate) second: KeyedRows,
    /// The work of the graph walks, its capacity kept across nodes and files.
    pub(crate) walk: WalkScratch,
    /// The work arrays of a cycle search, kept across files.
    pub(crate) cycles: crate::domain::cycles::types::CycleScratch,
}

/// What a filter decided before, for one node: the verdicts by text, kept for the run, and each name's decision, for the file.
#[derive(Debug, Default)]
pub(crate) struct NodeDecisions {
    /// The verdicts and compiled patterns of the node's comparisons.
    pub(crate) select: LazyScratch<SelectScratch>,
    /// What each comparison decided about each name of the open file.
    pub(crate) memo: Vec<Memo>,
}

/// Rows keyed by the hash of their key, with no allocation per row and no key copied out: each key's first and last row and
/// its row count in a hash table, and the next row of the same key in `next`, ascending, so a key's rows are read in source
/// order. A key is one name column's text, or the cells of key columns, compared from the keyed rows' own cells.
#[derive(Debug, Default)]
pub(crate) struct KeyedRows {
    /// Each key's hash, first row and last row.
    pub(crate) keys: hashbrown::HashTable<KeyedRun>,
    /// The next row of the same key after each row; `NO_ROW` after the last.
    pub(crate) next: Vec<IndexedRow>,
}

/// One key of an index: its hash, the first and last of its rows and how many rows it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyedRun {
    /// The hash of the key: of its text for a name column, of its cells otherwise.
    pub(crate) hash: IndexKeyHash,
    /// Its first row.
    pub(crate) first: IndexedRow,
    /// Its last row.
    pub(crate) last: IndexedRow,
    /// How many rows it has.
    pub(crate) rows: ryo_values::domain::rows::types::RowCount,
}

/// The hash of a key: of its text for a name column, of its cells otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IndexKeyHash(pub(crate) u64);

/// A row of an indexed relation, by its place in the relation's rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IndexedRow(pub(crate) u32);

/// The row after a key's last row.
pub(crate) const NO_ROW: IndexedRow = IndexedRow(u32::MAX);

impl FileBuffers {
    /// A node's rows, borrowed (cardinality included).
    #[must_use]
    pub fn rows(&self, node: PlanNodeId) -> NodeRows<'_> {
        usize::try_from(node.0).map_or(NodeRows::Absent, |at| {
            self.nodes
                .get(at)
                .and_then(Option::as_ref)
                .map_or(NodeRows::Absent, NodeRows::Materialized)
        })
    }

    /// The nodes that hold rows: the buffer high-water of a run, for the cases that bound it.
    #[cfg(test)]
    pub(crate) fn held(&self) -> impl Iterator<Item = PlanNodeId> + '_ {
        self.nodes
            .iter()
            .zip(0..)
            .filter(|(held, _)| held.is_some())
            .map(|(_, at)| PlanNodeId(at))
    }

    /// The node's last consumer is done: its buffer returns to the worker's pool.
    pub fn release(&mut self, node: PlanNodeId) {
        if let Ok(at) = usize::try_from(node.0)
            && let Some(held) = self.nodes.get_mut(at)
        {
            *held = None;
            if let Some(pending) = self.pending.get_mut(at) {
                *pending = None;
            }
        }
    }
}

/// The work of the graph walks: what one walk met and has left to expand, and, for a seeded closure, what each walked start
/// reached, so a start is walked once per file however many rows bind it.
#[derive(Debug, Default)]
pub(crate) struct WalkScratch {
    /// The entities the walk from one start has met.
    pub(crate) visited: hashbrown::HashTable<Value>,
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
    /// The rows a seeded closure reached, each as the join's logical right row, for the join's probe.
    pub(crate) rows: ryo_values::domain::rows::types::Table,
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
