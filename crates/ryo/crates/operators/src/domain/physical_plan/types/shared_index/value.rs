//! The probe indexes a plan shares, as one worker holds them for the open file: each index, the node whose release empties
//! it, and those the open file opened, which its end empties.
use crate::domain::kept_room::types::KeptRoom as _;
use crate::domain::physical_plan::types::keyed_rows::value::KeyedRows;
use crate::domain::physical_plan::types::touched_nodes::value::Touch;
use crate::domain::physical_plan::types::{IndexPlace, NodeReaders, PlanNodeId};

/// The probe indexes a plan shares, as one worker holds them.
#[derive(Debug, Default)]
pub(crate) struct SharedIndexes {
    /// Each index, by its place, its room kept across files within the cap.
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
    /// Empties the index for the next file, keeping its capacity.
    pub(crate) fn emptied(&mut self) {
        self.rows.cleared();
        self.remaining = NodeReaders(0);
    }
}

impl SharedIndexes {
    /// A probe of the index at `at`, of `readers` probes, runs: the first of the file counts every probe as still to come.
    pub(crate) fn opened(&mut self, at: IndexPlace, readers: NodeReaders) {
        if let Some(index) = self.indexes.get_mut(at.0)
            && index.remaining == NodeReaders(0)
        {
            index.remaining = readers;
            if index.listed == Touch::Untouched {
                index.listed = Touch::Touched;
                self.opened.push(at);
            }
        }
    }

    /// `node` is released: the index whose last probe it is is emptied.
    pub(crate) fn released(&mut self, node: PlanNodeId) {
        if let Some(Some(closed)) = self
            .closing
            .get(usize::try_from(node.0).unwrap_or(usize::MAX))
            && let Some(index) = self.indexes.get_mut(closed.0)
        {
            index.emptied();
        }
    }

    /// The open file ended: each index it opened is emptied, its room kept only within the cap, and no other is visited.
    pub(crate) fn ended(&mut self) {
        for place in self.opened.drain(..) {
            if let Some(index) = self.indexes.get_mut(place.0) {
                index.emptied();
                index.rows.kept();
                index.listed = Touch::Untouched;
            }
        }
    }
}
