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
    /// The rows `distinct` kept so far, keyed by all their cells in `hash_index`'s table, its capacity kept across nodes.
    pub(crate) repeats: KeyedRows,
    /// Each probe index the plan shares, by its place, its capacity kept across files.
    pub(crate) shared: Vec<SharedIndex>,
    /// How many times a probe built an index, its own or a shared one: what the sharing cases count.
    #[cfg(test)]
    pub(crate) builds: IndexBuilds,
}

/// How many probe indexes a worker built.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct IndexBuilds(pub(crate) usize);

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

    /// The node's last consumer is done: its buffer returns to the worker's pool. At the file's end, when the engine releases
    /// every node, a shared index whose last probe it is is emptied with it.
    pub fn release(&mut self, node: PlanNodeId) {
        for shared in &mut self.shared {
            if shared.last == node {
                shared.emptied();
            }
        }
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
        self.rows.keys.clear();
        self.rows.next.clear();
        self.remaining = NodeReaders(0);
    }
}
