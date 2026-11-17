//! Plan-time assignment and per-file lifetime of the keyed indexes declared by physical operators.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::row_set::value::RowSet;
use crate::domain::physical_plan::types::shared_index::value::IndexBuild;
use crate::domain::physical_plan::types::shared_index::value::{IndexKind, IndexRows, SharedIndex};
use crate::domain::physical_plan::types::{
    IndexIdentity, IndexPlace, KeyedIndex, KeyedIndexes, NodeReaders, PlanNodeId, ProbeIndex,
};

/// After rewrites, assigns a shared slot to compatible readers of the same source and key. Single readers keep their own index.
pub(crate) fn shared_indexes(nodes: &mut [PhysicalNode], indexes: &mut Vec<KeyedIndex>) {
    let mut keyed = KeyedIndexes::default();
    for (node, id) in nodes.iter_mut().zip((0..).map(PlanNodeId)) {
        let Some(reader) = node.lowering.index_reader() else {
            continue;
        };
        let _: IndexPlace = read_by(&mut keyed, reader.identity, id);
    }
    let keyed = keyed.indexes;
    for index in keyed.into_iter().filter(|index| index.readers.len() > 1) {
        let at = IndexPlace(indexes.len());
        for reader in &index.readers {
            if let Some(binding) = nodes
                .get_mut(usize::try_from(reader.0).unwrap_or(usize::MAX))
                .and_then(|node| node.lowering.index_reader())
            {
                *binding.slot = ProbeIndex::Shared(at);
            }
        }
        indexes.push(index);
    }
}

/// Records `reader` as a reader of the index `identity` names, met first now or before, and answers its place among `keyed`.
pub(crate) fn read_by(
    keyed: &mut KeyedIndexes,
    identity: IndexIdentity,
    reader: PlanNodeId,
) -> IndexPlace {
    let KeyedIndexes { indexes, places } = keyed;
    let at = *places.entry(identity).or_insert_with_key(|identity| {
        indexes.push(KeyedIndex {
            source: identity.source,
            key: identity.key.clone(),
            readers: Vec::new(),
        });
        IndexPlace(indexes.len() - 1)
    });
    if let Some(index) = indexes.get_mut(at.0) {
        index.readers.push(reader);
    }
    at
}

/// One shared index for each index of `plan`, each emptied by the release of its last probe: which node that is, is found
/// here, once, for every release after.
pub(crate) fn sized_shared(plan: &PhysicalPlan, buffers: &mut FileBuffers) {
    let shared = &mut buffers.shared;
    if shared.indexes.len() < plan.indexes.len() {
        // Each index keeps the kind of rows its key needs, decided here for every file.
        shared.indexes = plan
            .indexes
            .iter()
            .map(|index| SharedIndex {
                rows: match IndexKind::of_index(&index.key) {
                    IndexKind::Keyed => IndexRows::default(),
                    IndexKind::Rows => IndexRows::Rows(RowSet::default()),
                },
                ..SharedIndex::default()
            })
            .collect();
        shared.closing = vec![None; plan.nodes.len()];
        shared.opened.clear();
        for (at, index) in plan.indexes.iter().enumerate() {
            if let Some(last) = index.readers.last()
                && let Some(closing) = shared
                    .closing
                    .get_mut(usize::try_from(last.0).unwrap_or(usize::MAX))
            {
                *closing = Some(IndexPlace(at));
            }
        }
    }
}

/// A probe of shared index `at` runs: the first of the file counts every probe of it as still to come.
pub(crate) fn opened(plan: &PhysicalPlan, buffers: &mut FileBuffers, at: IndexPlace) {
    if let Some(index) = plan.indexes.get(at.0) {
        buffers.shared.opened(
            at,
            NodeReaders(u32::try_from(index.readers.len()).unwrap_or(u32::MAX)),
            &mut buffers.pools,
        );
    }
}

/// A probe of shared index `at` is done, whatever it answered: after the last of the file the index gives its rows to the
/// worker's index pool.
pub(crate) fn closed(buffers: &mut FileBuffers, at: IndexPlace) {
    if let Some(shared) = buffers.shared.indexes.get_mut(at.0) {
        shared.remaining = NodeReaders(shared.remaining.0.saturating_sub(1));
        if shared.remaining == NodeReaders(0) {
            shared.given_back(&mut buffers.pools);
        }
    }
}

/// Whether a probe of `index` builds the index it reads: its stage's own, at its first left row; a shared one, unless a probe
/// of the open file built it; none, for a probe that walks the tree.
pub(crate) fn probe_build(shared: &[SharedIndex], index: ProbeIndex) -> IndexBuild {
    match index {
        ProbeIndex::Own => IndexBuild::Builds,
        ProbeIndex::Tree => IndexBuild::Walked,
        // An index of no row is built again at no cost: an empty chain is an index not built in this file.
        ProbeIndex::Shared(at) => match shared.get(at.0) {
            Some(shared) if shared.rows.built_rows().0 > 0 => IndexBuild::Built,
            Some(_) | None => IndexBuild::Builds,
        },
    }
}

/// The index a probe of `index` reads: its stage's own `own`, or its shared index; a probe that walks the tree reads none
/// and is lent its own, unused.
///
/// # Errors
///
/// `WrongOperation` for a shared index the buffers do not hold.
pub(crate) fn probe_index<'b>(
    shared: &'b mut [SharedIndex],
    own: &'b mut IndexRows,
    index: ProbeIndex,
) -> Result<&'b mut IndexRows, PortableOperatorFault> {
    match index {
        ProbeIndex::Own | ProbeIndex::Tree => Ok(own),
        ProbeIndex::Shared(at) => shared
            .get_mut(at.0)
            .map(|shared| &mut shared.rows)
            .ok_or(PortableOperatorFault::WrongOperation),
    }
}
