//! One worker's node buffers: the rows of each materialized node of the open file, kept until its last consumer is done, and
//! their capacity kept across files.

use crate::domain::contract::types::{PortableOperatorFault, PredicateOutcome};
use crate::domain::kept_room::types::KeptRoom as _;
use crate::domain::physical_plan::types::keyed_rows::value::KeyedRows;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
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
    pub(crate) walk: crate::domain::physical_plan::types::walk_scratch::value::WalkScratch,
    /// The work arrays of a cycle search, kept across files.
    pub(crate) cycles: crate::domain::cycles::types::CycleScratch,
    /// The rows `distinct` kept so far, keyed by all their cells in `hash_index`'s table, its capacity kept across nodes.
    pub(crate) repeats: KeyedRows,
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
    /// How many times a probe built an index, its own or a shared one: what the sharing cases count.
    #[cfg(test)]
    pub(crate) builds: IndexBuilds,
    /// Whether a stage of a chain handed on a chunk past its bound: what the cell-bound case reads.
    #[cfg(test)]
    pub(crate) chunks: ChunkMeter,
    /// The node tables held: how many were formed, and the rows they held at once.
    #[cfg(test)]
    pub(crate) meter: TableMeter,
    /// How many times a chain called a stage: what the chunk-linearity cases read.
    #[cfg(test)]
    pub(crate) stage_calls: StageCalls,
    /// The work the ends of files did: what the release-cost case reads.
    #[cfg(test)]
    pub(crate) ending: EndWork,
}

/// The work the releases and the ends of a worker's files did: each node released, each shared index it visited.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct EndWork(pub(crate) usize);

/// How many times a worker's chains called a stage, each call a run of rows.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StageCalls(pub(crate) usize);

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

/// What a filter decided before, for one node: the verdicts by text, kept for the run, and each name's decision, for the file.
#[derive(Debug, Default)]
pub(crate) struct NodeDecisions {
    /// The verdicts and compiled patterns of the node's comparisons.
    pub(crate) select: LazyScratch<SelectScratch>,
    /// What each comparison decided about each name of the open file.
    pub(crate) memo: MemoTable,
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

    /// The node's last consumer is done: its buffer returns to the worker's pool, and a shared index whose last probe it is is
    /// emptied with it.
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
        self.shared.released(node);
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
        // Made or not, the node keeps nothing it was done with into the next file.
        self.done.forgotten(node);
    }
}

impl FileBuffers {
    /// Ends the open file: each node it touched is released and each shared index it opened emptied, no other, so the work is
    /// the file's, not the program's; its scratch keeps its room for the next file only within the cap.
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
        self.shared.ended();
        self.index.kept();
        self.second.kept();
        self.repeats.kept();
        self.stream.kept();
        self.values.kept();
        self.hashes.0.kept();
        self.found.kept();
        self.walk.kept();
        self.cycles.kept();
        self.worklist.kept();
    }
}
