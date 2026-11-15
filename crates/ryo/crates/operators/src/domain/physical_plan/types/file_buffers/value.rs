//! One worker's node buffers: the rows of each materialized node of the open file, kept until its last consumer is done, and
//! their capacity kept across files.

use crate::domain::contract::types::{PortableOperatorFault, PredicateOutcome};
use crate::domain::physical_plan::types::keyed_rows::value::{IndexKeyHash, IndexedRow, KeyedRows};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{EngineUses, NodeFed, NodeReaders, NodeRows, PlanNodeId};
use crate::domain::table_scratch::types::{LazyScratch, Memo, SelectScratch};
use ryo_relations::domain::relations::types::{NodeId, Value};

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
    /// The rows each node's kernels read from each input when it was made in the open file, by node id.
    pub(crate) fed: Vec<NodeFed>,
    /// How many of each node's readers have not read it yet in this file, by node id.
    pub(crate) remaining: Vec<NodeReaders>,
    /// How many of each node's engine uses are not done yet in this file, by node id.
    pub(crate) engine_left: Vec<EngineUses>,
    /// The emptied node tables kept for the next nodes, capped.
    pub(crate) pool: crate::domain::physical_plan::types::table_pool::value::TablePool,
    /// Each filtering node's comparison scratch, by node id, kept across files.
    pub(crate) decisions: Vec<NodeDecisions>,
    /// The keyed-row table a node builds for itself: a probe's own index, a group's, or the lookups' while it answers them.
    pub(crate) index: KeyedRows,
    /// The row a kernel is reading, decoded from the facts one row at a time.
    pub(crate) row: Vec<Value>,
    /// The row a kernel is writing.
    pub(crate) written: Vec<Value>,
    /// The values a kernel collects: an extension's resolved values, a group's distinct values.
    pub(crate) values: Vec<Value>,
    /// The row and text buffers of a text kind's rows, their capacity kept across files.
    pub(crate) scratch: crate::domain::contract::types::RowScratch,
    /// The places of a text kind's slots or words in their text, their capacity kept across files.
    pub(crate) places: crate::domain::contract::types::RangeScratch,
    /// The hashes of a group's values, which a group digest folds.
    pub(crate) hashes: crate::domain::reduction::types::GroupHashes,
    /// The second keyed-row table a kernel reads beside `index`: an axis's domain. The same table, a second instance.
    pub(crate) second: KeyedRows,
    /// The work of the graph walks, its capacity kept across nodes and files.
    pub(crate) walk: WalkScratch,
    /// The work arrays of a cycle search, kept across files.
    pub(crate) cycles: crate::domain::cycles::types::CycleScratch,
    /// The rows `distinct` kept so far, keyed by all their cells in `hash_index`'s table, its capacity kept across nodes.
    pub(crate) repeats: KeyedRows,
    /// The tree nodes one left row of a tree probe found, its capacity kept across files.
    pub(crate) found: Vec<NodeId>,
    /// Each probe index the plan shares, by its place, its capacity kept across files.
    pub(crate) shared: Vec<SharedIndex>,
    /// The work of the chains of streamed stages, its capacity kept across nodes and files.
    pub(crate) stream: crate::domain::physical_plan::types::stream_scratch::value::StreamScratch,
    /// How many times a probe built an index, its own or a shared one: what the sharing cases count.
    #[cfg(test)]
    pub(crate) builds: IndexBuilds,
    /// Whether a stage of a chain handed on a chunk past its bound: what the cell-bound case reads.
    #[cfg(test)]
    pub(crate) chunks: ChunkMeter,
    /// The node tables held: how many were formed, and the rows they held at once.
    #[cfg(test)]
    pub(crate) meter: TableMeter,
}

/// How many probe indexes a worker built.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct IndexBuilds(pub(crate) usize);

/// The node tables a worker held: how many it formed, the rows they hold now, and the most rows they held at once.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TableMeter {
    /// How many node tables were held.
    pub(crate) formed: TablesFormed,
    /// The rows the held node tables hold now.
    pub(crate) live_rows: ryo_values::domain::rows::types::RowCount,
    /// The most rows the held node tables held at once.
    pub(crate) peak_rows: ryo_values::domain::rows::types::RowCount,
}

/// How many node tables a worker held.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TablesFormed(pub(crate) u32);

#[cfg(test)]
impl TableMeter {
    /// A node table of `rows` rows is held.
    pub(crate) fn formed(&mut self, rows: ryo_values::domain::rows::types::RowCount) {
        self.formed.0 += 1;
        self.live_rows.0 += rows.0;
        self.peak_rows.0 = self.peak_rows.0.max(self.live_rows.0);
    }

    /// A held node table of `rows` rows is dropped.
    pub(crate) fn dropped(&mut self, rows: ryo_values::domain::rows::types::RowCount) {
        self.live_rows.0 = self.live_rows.0.saturating_sub(rows.0);
    }
}

/// Whether every chunk a worker's chains handed on held less than the bound before the input row that wrote its last rows.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ChunkMeter {
    /// Every chunk did.
    #[default]
    Within,
    /// A chunk held the bound's cells or more before it.
    Over,
}

/// A probe index the plan shares: built by the first of its probes in a file to read its rows, and emptied after the last,
/// or when the engine releases its last probe at the file's end, so no file reads another file's index.
#[derive(Debug)]
pub(crate) struct SharedIndex {
    /// The keyed rows; empty until a probe of the open file builds them.
    pub(crate) rows: KeyedRows,
    /// How many of its probes have not run in the open file; none before its first.
    pub(crate) remaining: NodeReaders,
    /// Its last probe in node order.
    pub(crate) last: PlanNodeId,
}

/// What a filter decided before, for one node: the verdicts by text, kept for the run, and each name's decision, for the file.
#[derive(Debug, Default)]
pub(crate) struct NodeDecisions {
    /// The verdicts and compiled patterns of the node's comparisons.
    pub(crate) select: LazyScratch<SelectScratch>,
    /// What each comparison decided about each name of the open file.
    pub(crate) memo: Vec<Memo>,
}

impl FileBuffers {
    /// A node's rows, borrowed (cardinality included).
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
    /// use ryo_operators::domain::physical_plan::types::{NodeRows, PlanNodeId};
    /// assert!(matches!(FileBuffers::default().rows(PlanNodeId(0)), NodeRows::Absent));
    /// ```
    #[must_use]
    pub fn rows(&self, node: PlanNodeId) -> NodeRows<'_> {
        usize::try_from(node.0).map_or(NodeRows::Absent, |at| {
            self.nodes
                .get(at)
                .and_then(Option::as_ref)
                .map_or(NodeRows::Absent, NodeRows::Materialized)
        })
    }

    /// The rows `node`'s kernels read from each input when it was made in the open file; none for a node not made or
    /// released.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
    /// use ryo_operators::domain::physical_plan::types::{NodeFed, PlanNodeId};
    /// assert_eq!(FileBuffers::default().fed(PlanNodeId(0)), NodeFed::default());
    /// ```
    #[must_use]
    pub fn fed(&self, node: PlanNodeId) -> NodeFed {
        usize::try_from(node.0).map_or(NodeFed::default(), |at| {
            self.fed.get(at).copied().unwrap_or_default()
        })
    }

    /// Keeps of `node`'s rows those `keep` holds for, in their order, and drops the rest. This is the engine's one
    /// change to rows the operators lent it: after the files, the rows an antijoin or a difference answered from the absence
    /// of rows that a partial operand could change are dropped where the step stands, before any later step reads them, as
    /// the coverage filtering of the interpreter did. A node that holds no rows is left as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
    /// use ryo_operators::domain::physical_plan::types::{NodeRows, PlanNodeId};
    /// use ryo_operators::domain::contract::types::PredicateOutcome;
    /// let mut buffers = FileBuffers::default();
    /// buffers.retain(PlanNodeId(0), |_| PredicateOutcome::Fails);
    /// assert_eq!(buffers.rows(PlanNodeId(0)), NodeRows::Absent);
    /// ```
    pub fn retain(&mut self, node: PlanNodeId, mut keep: impl FnMut(&[Value]) -> PredicateOutcome) {
        let Some(Some(table)) = self
            .nodes
            .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
        else {
            return;
        };
        let width = table.kinds.len();
        let mut written = 0;
        let mut kept = 0;
        for row in 0..table.count.0 {
            let start = row.saturating_mul(width);
            let end = start.saturating_add(width);
            let Some(cells) = table.cells.get(start..end) else {
                break;
            };
            if keep(cells) == PredicateOutcome::Holds {
                table.cells.copy_within(start..end, written);
                written = written.saturating_add(width);
                kept += 1;
            }
        }
        table.cells.truncate(written);
        #[cfg(test)]
        self.meter
            .dropped(ryo_values::domain::rows::types::RowCount(
                table.count.0 - kept,
            ));
        table.count = ryo_values::domain::rows::types::RowCount(kept);
    }

    /// One engine use of `node` is done: a rule that emits it, or a consumer that holds it, is finished with it. Its rows go
    /// once no engine use and no node reader of it remains in the open file, whichever comes last; a node the file holds no
    /// rows of is left as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
    /// use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
    /// use ryo_operators::domain::physical_plan::types::{NodeRows, PlanNodeId};
    /// let mut buffers = FileBuffers::default();
    /// buffers.used(&PhysicalPlan::default(), PlanNodeId(0));
    /// assert_eq!(buffers.rows(PlanNodeId(0)), NodeRows::Absent);
    /// ```
    pub fn used(&mut self, plan: &PhysicalPlan, node: PlanNodeId) {
        let at = usize::try_from(node.0).unwrap_or(usize::MAX);
        // A node of another plan has no engine use here.
        if plan.nodes.get(at).is_none() {
            return;
        }
        // A node the open file never made, or has already given back, holds no rows: its use releases nothing, so a shared
        // index whose last probe it is stays for the probe that makes it later in the file.
        if self.nodes.get(at).is_none_or(Option::is_none) {
            return;
        }
        let Some(left) = self.engine_left.get_mut(at) else {
            return;
        };
        left.0 = left.0.saturating_sub(1);
        if left.0 == 0 && self.remaining.get(at).is_none_or(|readers| readers.0 == 0) {
            self.release(node);
        }
    }

    /// The node's last consumer is done: its buffer returns to the worker's pool. At the file's end, when the engine releases
    /// every node, a shared index whose last probe it is is emptied with it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
    /// use ryo_operators::domain::physical_plan::types::{NodeRows, PlanNodeId};
    /// let mut buffers = FileBuffers::default();
    /// buffers.release(PlanNodeId(0));
    /// assert!(matches!(buffers.rows(PlanNodeId(0)), NodeRows::Absent));
    /// ```
    pub fn release(&mut self, node: PlanNodeId) {
        for shared in &mut self.shared {
            if shared.last == node {
                shared.emptied();
            }
        }
        if let Ok(at) = usize::try_from(node.0)
            && let Some(held) = self.nodes.get_mut(at)
        {
            if let Some(table) = held.take() {
                #[cfg(test)]
                self.meter.dropped(table.count);
                self.pool.pooled(table);
            }
            if let Some(pending) = self.pending.get_mut(at) {
                *pending = None;
            }
            if let Some(fed) = self.fed.get_mut(at) {
                *fed = NodeFed::default();
            }
            if let Some(left) = self.engine_left.get_mut(at) {
                *left = EngineUses(0);
            }
        }
    }
}

impl SharedIndex {
    /// The index of no file yet: its last probe is `last`.
    pub(crate) fn new(last: PlanNodeId) -> Self {
        Self {
            rows: KeyedRows::default(),
            remaining: NodeReaders(0),
            last,
        }
    }

    /// Empties the index for the next file, keeping its capacity.
    pub(crate) fn emptied(&mut self) {
        self.rows.cleared();
        self.remaining = NodeReaders(0);
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
