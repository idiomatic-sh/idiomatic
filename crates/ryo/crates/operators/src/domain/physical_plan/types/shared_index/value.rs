//! The probe indexes a plan shares, as one worker holds them for the open file: each index, the node whose release empties
//! it, and those the open file opened, which its end empties.
use crate::domain::equality_join::types::ProbeKey;
use crate::domain::kept_room::constants::POOL_INDEXES;
use crate::domain::kept_room::types::PoolSize;
use crate::domain::kept_room::types::kept_pool::value::KeptPool;
use crate::domain::physical_plan::types::keyed_rows::value::KeyedRows;
use crate::domain::physical_plan::types::row_set::value::RowSet;
use crate::domain::physical_plan::types::touched_nodes::value::Touch;
use crate::domain::physical_plan::types::{IndexKey, IndexPlace, NodeReaders, PlanNodeId};

/// The emptied index tables a worker keeps for its next indexes and repeats, by kind: keyed-row tables for indexes of a key,
/// row sets for rows kept once. Each kind is given back to its own pool.
#[derive(Debug, Default)]
pub(crate) struct IndexPools {
    /// The keyed-row tables.
    pub(crate) keyed: KeptPool<KeyedRows>,
    /// The row sets.
    pub(crate) rows: KeptPool<RowSet>,
}

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
    /// The rows, of the kind its key needs; empty until a probe of the open file builds them.
    pub(crate) rows: IndexRows,
    /// How many of its probes have not run in the open file; none before its first.
    pub(crate) remaining: NodeReaders,
    /// Whether the open file opened it: listed once among those its end empties.
    pub(crate) listed: Touch,
}

/// The kind of rows an index keeps: keyed rows for a key of columns or of a name's text, a row set for a whole-row key, which
/// is asked only whether a row is there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IndexKind {
    /// Keyed rows: each key's rows, in order.
    Keyed,
    /// A row set: each row once, by all its cells.
    Rows,
}

impl IndexKind {
    /// The kind a probe of `key` reads: a row set for a whole-row key, keyed rows otherwise.
    pub(crate) fn of_probe(key: &ProbeKey) -> Self {
        match key {
            ProbeKey::Whole => Self::Rows,
            ProbeKey::Columns { .. } => Self::Keyed,
        }
    }

    /// The kind a shared index of `key` keeps.
    pub(crate) fn of_index(key: &IndexKey) -> Self {
        match key {
            IndexKey::Rows => Self::Rows,
            IndexKey::Text(_) | IndexKey::Cells(_) => Self::Keyed,
        }
    }
}

/// The rows of an index, of the kind its key needs: one storage for a shared index, a probe's own index and the held
/// probe's, each kind taken from and given back to its own pool.
#[derive(Debug)]
pub(crate) enum IndexRows {
    /// Keyed rows.
    Keyed(KeyedRows),
    /// A row set.
    Rows(RowSet),
}

impl Default for IndexRows {
    fn default() -> Self {
        Self::Keyed(KeyedRows::default())
    }
}

impl IndexRows {
    /// Its kind.
    pub(crate) fn kind(&self) -> IndexKind {
        match self {
            Self::Keyed(_) => IndexKind::Keyed,
            Self::Rows(_) => IndexKind::Rows,
        }
    }

    /// Readies room of `kind` for a build about to fill it: its own room, emptied, when it is of that kind and holds some;
    /// else room from the pool of that kind, its room of the other kind given to that kind's pool first.
    pub(crate) fn readied(&mut self, kind: IndexKind, pools: &mut IndexPools) {
        if self.kind() != kind {
            self.given_back(pools);
            *self = match kind {
                IndexKind::Keyed => Self::Keyed(pools.keyed.fresh()),
                IndexKind::Rows => Self::Rows(pools.rows.fresh()),
            };
        }
        match self {
            Self::Keyed(rows) => pools.keyed.refilled(rows),
            Self::Rows(set) => pools.rows.refilled(set),
        }
    }

    /// Gives its room to the pool of its kind, which keeps it only within its caps, and is left of its kind with no room.
    pub(crate) fn given_back(&mut self, pools: &mut IndexPools) {
        match self {
            Self::Keyed(rows) => pools
                .keyed
                .pooled(std::mem::take(rows), PoolSize(POOL_INDEXES)),
            Self::Rows(set) => pools
                .rows
                .pooled(std::mem::take(set), PoolSize(POOL_INDEXES)),
        }
    }

    /// Empties it, keeping its room and its kind.
    pub(crate) fn cleared(&mut self) {
        match self {
            Self::Keyed(rows) => rows.cleared(),
            Self::Rows(set) => set.cleared(),
        }
    }

    /// How many rows it was built from, as its kind counts them: a key's runs of keyed rows, the rows of a row set. None for
    /// an index no probe of the open file built, or one built from no row.
    pub(crate) fn built_rows(&self) -> ryo_values::domain::rows::types::RowCount {
        match self {
            Self::Keyed(rows) => ryo_values::domain::rows::types::RowCount(rows.next.len()),
            Self::Rows(set) => set.count(),
        }
    }
}

impl IndexRows {
}

impl SharedIndex {
    /// Gives its rows to the worker's index pool of their kind, which keeps their room only within its caps, and counts no
    /// probe still to come: the next probe takes room from the pool and builds it anew.
    pub(crate) fn given_back(&mut self, pools: &mut IndexPools) {
        self.rows.given_back(pools);
        self.remaining = NodeReaders(0);
    }
}

impl SharedIndexes {
    /// A probe of the index at `at`, of `readers` probes, runs: the first of the file counts every probe as still to come,
    /// and readies the index's rows with room from the worker's index pool.
    pub(crate) fn opened(&mut self, at: IndexPlace, readers: NodeReaders, pools: &mut IndexPools) {
        if let Some(index) = self.indexes.get_mut(at.0)
            && index.remaining == NodeReaders(0)
        {
            let kind = index.rows.kind();
            index.rows.readied(kind, pools);
            index.remaining = readers;
            if index.listed == Touch::Untouched {
                index.listed = Touch::Touched;
                self.opened.push(at);
            }
        }
    }

    /// `node` is released: the index whose last probe it is gives its rows to the worker's index pool.
    pub(crate) fn released(&mut self, node: PlanNodeId, pools: &mut IndexPools) {
        if let Some(Some(closed)) = self
            .closing
            .get(usize::try_from(node.0).unwrap_or(usize::MAX))
            && let Some(index) = self.indexes.get_mut(closed.0)
        {
            index.given_back(pools);
        }
    }

    /// The open file ended: each index it opened gives its rows to the worker's index pool, and no other is visited.
    pub(crate) fn ended(&mut self, pools: &mut IndexPools) {
        for place in self.opened.drain(..) {
            if let Some(index) = self.indexes.get_mut(place.0) {
                index.given_back(pools);
                index.listed = Touch::Untouched;
            }
        }
    }
}

/// A reader of one planned index, with the number of readers the file must account for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IndexReader {
    /// Which storage the planner assigned.
    pub(crate) place: IndexPlace,
    /// How many nodes can read this identity in the file.
    pub(crate) readers: NodeReaders,
}

/// Storage for one ordered key over its original source rows. Only demand from a reader opens shared storage.
#[derive(Debug)]
pub(crate) enum KeyedStorage<'a> {
    /// The node's own scratch.
    Own(&'a mut KeyedRows),
    /// The unchanged published relation and exact ordered key have more than one reader.
    File {
        /// The file-local owner.
        indexes: &'a mut SharedIndexes,
        /// Where the owner borrows empty storage.
        pools: &'a mut IndexPools,
        /// The planner's reader registration.
        reader: IndexReader,
    },
}

/// An access to keyed source rows, measured at the build in tests.
#[derive(Debug)]
pub(crate) struct KeyedAccess<'a> {
    /// Where the key's ordered row links live.
    pub(crate) storage: KeyedStorage<'a>,
}

impl<'b> KeyedAccess<'b> {
    /// The ordered row links a demanded key reads. A failed build leaves no reusable prefix.
    ///
    /// # Errors
    ///
    /// The first fault while building the existing source-row index, or a mismatched index kind.
    pub(crate) fn rows(
        self,
        build: impl FnOnce(
            &mut KeyedRows,
        ) -> Result<(), crate::domain::contract::types::PortableOperatorFault>,
    ) -> Result<&'b KeyedRows, crate::domain::contract::types::PortableOperatorFault> {
        use crate::domain::contract::types::PortableOperatorFault;
        let rows = match self.storage {
            KeyedStorage::Own(rows) => rows,
            KeyedStorage::File {
                indexes,
                pools,
                reader,
            } => {
                indexes.opened(reader.place, reader.readers, pools);
                let stored = indexes
                    .indexes
                    .get_mut(reader.place.0)
                    .ok_or(PortableOperatorFault::WrongOperation)?;
                let IndexRows::Keyed(rows) = &mut stored.rows else {
                    return Err(PortableOperatorFault::WrongOperation);
                };
                if !rows.next.is_empty() {
                    return Ok(rows);
                }
                rows
            }
        };
        if let Err(fault) = build(rows) {
            rows.cleared();
            return Err(fault);
        }
        Ok(rows)
    }
}
