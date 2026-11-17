//! The probe indexes a plan shares, as one worker holds them for the open file: each index, the node whose release empties
//! it, and those the open file opened, which its end empties.
use crate::domain::kept_room::constants::POOL_INDEXES;
use crate::domain::kept_room::types::PoolSize;
use crate::domain::kept_room::types::kept_pool::value::KeptPool;
use crate::domain::physical_plan::types::keyed_rows::value::KeyedRows;
use crate::domain::physical_plan::types::touched_nodes::value::Touch;
use crate::domain::physical_plan::types::{IndexPlace, NodeReaders, PlanNodeId};

/// The probe indexes a plan shares, as one worker holds them.
#[derive(Debug, Default)]
pub(crate) struct SharedIndexes {
    /// Each index, by its place: it holds room only while the open file's probes read it.
    pub(crate) indexes: Vec<SharedIndex>,
    /// The index each node is the last probe of, by node id: found once, as the indexes are sized for the plan.
    pub(crate) closing: Vec<Option<IndexPlace>>,
    /// The indexes the open file's probes opened, each once: the indexes its end empties.
    pub(crate) opened: Vec<IndexPlace>,
}

/// A probe index the plan shares: built by the first of its probes in a file to read its rows, and emptied after the last,
/// when the engine releases its last probe, or at the file's end, so no file reads another file's index.
#[derive(Debug, Default)]
pub(crate) struct SharedIndex {
    /// The keyed rows; empty until a probe of the open file builds them.
    pub(crate) rows: KeyedRows,
    /// How many of its probes have not run in the open file; none before its first.
    pub(crate) remaining: NodeReaders,
    /// Whether the open file opened it: listed once among those its end empties.
    pub(crate) listed: Touch,
}

impl SharedIndex {
    /// Gives its rows to the worker's index pool, which keeps their room only within its caps, and counts no probe still to
    /// come: the next probe takes room from the pool and builds it anew.
    pub(crate) fn given_back(&mut self, indexes: &mut KeptPool<KeyedRows>) {
        let _ = (indexes, PoolSize(POOL_INDEXES));
        self.rows.cleared();
        self.remaining = NodeReaders(0);
    }
}

impl SharedIndexes {
    /// A probe of the index at `at`, of `readers` probes, runs: the first of the file counts every probe as still to come,
    /// and readies the index's rows with room from the worker's index pool.
    pub(crate) fn opened(
        &mut self,
        at: IndexPlace,
        readers: NodeReaders,
        indexes: &mut KeptPool<KeyedRows>,
    ) {
        if let Some(index) = self.indexes.get_mut(at.0)
            && index.remaining == NodeReaders(0)
        {
            indexes.refilled(&mut index.rows);
            index.remaining = readers;
            if index.listed == Touch::Untouched {
                index.listed = Touch::Touched;
                self.opened.push(at);
            }
        }
    }

    /// `node` is released: the index whose last probe it is gives its rows to the worker's index pool.
    pub(crate) fn released(&mut self, node: PlanNodeId, indexes: &mut KeptPool<KeyedRows>) {
        if let Some(Some(closed)) = self
            .closing
            .get(usize::try_from(node.0).unwrap_or(usize::MAX))
            && let Some(index) = self.indexes.get_mut(closed.0)
        {
            index.given_back(indexes);
        }
    }

    /// The open file ended: each index it opened gives its rows to the worker's index pool, and no other is visited.
    pub(crate) fn ended(&mut self, indexes: &mut KeptPool<KeyedRows>) {
        for place in self.opened.drain(..) {
            if let Some(index) = self.indexes.get_mut(place.0) {
                index.given_back(indexes);
                crate::domain::kept_room::types::KeptRoom::kept(&mut index.rows);
                index.listed = Touch::Untouched;
            }
        }
    }
}
