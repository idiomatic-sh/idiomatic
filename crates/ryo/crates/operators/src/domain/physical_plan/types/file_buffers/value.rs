//! One worker's node buffers: the rows of each materialized node of the open file, kept until its last consumer is done, and
//! their capacity kept across files.

use crate::domain::contract::types::{PortableOperatorFault, PredicateOutcome};
use crate::domain::kept_room::constants::{POOL_INDEXES, POOL_TABLES};
use crate::domain::kept_room::types::KeptRoom as _;
use crate::domain::kept_room::types::PoolSize;
use crate::domain::kept_room::types::kept_pool::value::KeptPool;
use crate::domain::physical_plan::types::keyed_rows::value::KeyedRows;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::shared_index::value::IndexRows;
#[cfg(test)]
use crate::domain::physical_plan::types::worker_meters::value::{
    ChunkMeter, EndWork, IndexBuilds, TableMeter,
};
use crate::domain::physical_plan::types::{EngineUses, NodeFed, NodeReaders, NodeRows, PlanNodeId};
use crate::domain::table_scratch::types::{LazyScratch, MemoTable, SelectScratch};
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
    /// Optional immutable query state belongs to exactly one open file.
    pub(crate) entity_queries: crate::domain::entity_queries::types::entity_files::value::EntityFileSession,
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
    /// The readers and engine uses of each node the open file is done with before it made the node.
    pub(crate) done: crate::domain::physical_plan::types::consumers_done::value::ConsumersDone,
    /// The emptied node tables kept for the next nodes, capped.
    pub(crate) pool: KeptPool<ryo_values::domain::rows::types::Table>,
    /// The emptied keyed-row tables and row sets kept for the next indexes and repeats, shared and streamed alike, each kind
    /// in its own pool, capped.
    pub(crate) pools: crate::domain::physical_plan::types::shared_index::value::IndexPools,
    /// Each filtering node's comparison scratch, by node id, kept across files.
    pub(crate) decisions: Vec<NodeDecisions>,
    /// The keyed-row table a node builds for itself: a group's, a closure's or a cycle search's edges, or the lookups' while
    /// it answers them; readied from the keyed pool and given back to it at the file's end.
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
    /// The second index a kernel reads beside `index`, of the kind it needs: a seeded closure's held probe's own index, or
    /// an axis's domain, keyed; given back to the pool of its kind at the file's end.
    pub(crate) second: IndexRows,
    /// The work of the graph walks, its capacity kept across nodes and files.
    pub(crate) walk: crate::domain::physical_plan::types::walk_scratch::value::WalkScratch,
    /// The work arrays of a cycle search, kept across files.
    pub(crate) cycles: crate::domain::cycles::types::CycleScratch,
    /// The rows `distinct`, a seeded closure's probe or a worklist kept so far, once each; readied from the row-set pool and
    /// given back to it at the file's end.
    pub(crate) repeats: crate::domain::physical_plan::types::row_set::value::RowSet,
    /// The tree nodes one left row of a tree probe found, its capacity kept across files.
    pub(crate) found: Vec<NodeId>,
    /// The probe indexes the plan shares, those the open file opened, and the node whose release empties each.
    pub(crate) shared: crate::domain::physical_plan::types::shared_index::value::SharedIndexes,
    /// The nodes the open file gave rows or a feed: the nodes its end releases.
    pub(crate) touched: crate::domain::physical_plan::types::touched_nodes::value::TouchedNodes,
    /// The work of the chains of streamed stages, its capacity kept across nodes and files.
    pub(crate) stream: crate::domain::physical_plan::types::stream_scratch::value::StreamScratch,
    /// The work of a worklist, its capacity kept across nodes and files.
    pub(crate) worklist: crate::domain::worklist::types::WorklistScratch,
    /// What each node did over the files, when counting is on; off, nothing is counted or timed.
    pub(crate) counters:
        Option<crate::domain::physical_plan::types::node_counters::value::NodeCounters>,
    /// How many times a probe built an index, its own or a shared one: what the sharing cases count.
    #[cfg(test)]
    pub(crate) builds: IndexBuilds,
    /// Whether a stage of a chain handed on a chunk past its bound: what the cell-bound case reads.
    #[cfg(test)]
    pub(crate) chunks: ChunkMeter,
    /// The node tables held: how many were formed, and the rows they held at once.
    #[cfg(test)]
    pub(crate) meter: TableMeter,
    /// The work the chains did: what the chunk-linearity and whole-row cases read.
    #[cfg(test)]
    pub(crate) chains: crate::domain::physical_plan::types::stream_scratch::value::ChainWork,
    /// The work the ends of files did: what the release-cost case reads.
    #[cfg(test)]
    pub(crate) ending: EndWork,
}

/// What a filter decided before, for one node: the verdicts by text, kept for the run, and each name's decision, for the file.
#[derive(Debug, Default)]
pub(crate) struct NodeDecisions {
    /// The verdicts and compiled patterns of the node's comparisons.
    pub(crate) select: LazyScratch<SelectScratch>,
    /// What each comparison decided about each name of the open file.
    pub(crate) memo: MemoTable,
}

impl FileBuffers {
    /// Counts what each node does from now on: how many times it is made, the rows it reads and writes and the time it
    /// takes, over every file these buffers run, until the counts are taken.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
    /// let mut buffers = FileBuffers::default();
    /// buffers.count_nodes();
    /// assert!(buffers.take_counters().is_some());
    /// ```
    pub fn count_nodes(&mut self) {
        let _counting = self.counters.get_or_insert_default();
    }

    /// The counts of the nodes made since counting was switched on, which ends it; none when it was off.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
    /// assert!(FileBuffers::default().take_counters().is_none());
    /// ```
    pub fn take_counters(
        &mut self,
    ) -> Option<crate::domain::physical_plan::types::node_counters::value::NodeCounters> {
        self.counters.take()
    }

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
    /// once no engine use and no node reader of it remains in the open file, whichever comes last; a use before it is made
    /// is counted when it is made.
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
        // A node not made yet counts the use when it is made.
        if self.nodes.get(at).is_none_or(Option::is_none) {
            return self.done.engine_done(plan, node);
        }
        let Some(left) = self.engine_left.get_mut(at) else {
            return;
        };
        left.0 = left.0.saturating_sub(1);
        if left.0 == 0 && self.remaining.get(at).is_none_or(|readers| readers.0 == 0) {
            self.release(node);
        }
    }

    /// `node` does not run in the open file: each read it makes of an earlier node's rows is done. Each is counted when its
    /// source is made, so this comes before the file makes any node: at the file's start.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
    /// use ryo_operators::domain::physical_plan::types::{PlanNodeId, physical_plan::value::PhysicalPlan};
    /// FileBuffers::default().unread(&PhysicalPlan::default(), PlanNodeId(0));
    /// ```
    pub fn unread(&mut self, plan: &PhysicalPlan, node: PlanNodeId) {
        self.done.unread(plan, node);
    }

    /// A node that holds no row takes `kinds` as its columns; a node with rows, or with none made, is left as it is. This is the
    /// engine's other change to rows the operators lent it: a step whose schema refusal is deferred to its rows answers no
    /// column to a step reading it, as the interpreter's reader of a refused step read none, so a reader of two such steps
    /// meets the same columns on both sides.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
    /// use ryo_operators::domain::physical_plan::types::{NodeRows, PlanNodeId};
    /// let mut buffers = FileBuffers::default();
    /// buffers.empty_as(PlanNodeId(0), &[]);
    /// assert_eq!(buffers.rows(PlanNodeId(0)), NodeRows::Absent);
    /// ```
    pub fn empty_as(
        &mut self,
        node: PlanNodeId,
        kinds: &[ryo_relations::domain::relations::types::CellKind],
    ) {
        if let Some(Some(table)) = self
            .nodes
            .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
            && table.count.0 == 0
        {
            table.kinds.clear();
            table.kinds.extend_from_slice(kinds);
            table.cells.clear();
        }
    }

    /// The node's last consumer is done: its buffer returns to the worker's pool, and a shared index whose last probe it is
    /// gives its rows to the worker's index pool with it.
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
        #[cfg(test)]
        {
            self.ending.0 += 1;
        }
        self.shared.released(node, &mut self.pools);
        if let Ok(at) = usize::try_from(node.0)
            && let Some(held) = self.nodes.get_mut(at)
        {
            if let Some(table) = held.take() {
                #[cfg(test)]
                self.meter.dropped(table.count);
                self.pool.pooled(table, PoolSize(POOL_TABLES));
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
        // Made or not, the node keeps nothing it was done with into the next file.
        self.done.forgotten(node);
    }
}

impl FileBuffers {

    /// Ends the open file: each node it touched is released, and each shared index it opened, each streamed stage's own
    /// index and repeats, and the worker's own index, second index and repeats give their rows to the worker's index pools,
    /// each to the pool of its kind, no other, so the work is the file's, not the program's; its other scratch keeps its room
    /// for the next file only within the cap.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
    /// use ryo_operators::domain::physical_plan::types::{NodeRows, PlanNodeId};
    /// let mut buffers = FileBuffers::default();
    /// buffers.ended();
    /// assert_eq!(buffers.rows(PlanNodeId(0)), NodeRows::Absent);
    /// ```
    pub fn ended(&mut self) {
        crate::spike::flush();
        self.entity_queries = Default::default();
        let touched = self.touched.taken();
        for node in &touched {
            self.release(*node);
            if let Some(decisions) = self
                .decisions
                .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
            {
                decisions.memo.kept();
            }
        }
        self.touched.given_back(touched);
        self.done.file_ended();
        #[cfg(test)]
        {
            self.ending.0 += self.shared.opened.len();
        }
        self.shared.ended(&mut self.pools);
        self.pools
            .keyed
            .pooled(std::mem::take(&mut self.index), PoolSize(POOL_INDEXES));
        self.second.given_back(&mut self.pools);
        self.pools
            .rows
            .pooled(std::mem::take(&mut self.repeats), PoolSize(POOL_INDEXES));
        self.stream.given_back(&mut self.pools);
        self.stream.kept();
        self.values.kept();
        self.hashes.0.kept();
        self.found.kept();
        self.walk.kept();
        self.cycles.kept();
        self.worklist.kept();
    }
}
