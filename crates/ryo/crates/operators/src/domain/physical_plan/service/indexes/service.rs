//! The probe indexes a plan shares: the planning step that shares one `hash_index` between the probes of one right side and
//! key, decided once as the plan is built; and each shared index's life in a file, from its first probe to its last.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::{ProbeKey, ProbeMode};
use crate::domain::physical_plan::collections::kernels::probe_right::item::IndexBuild;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::keyed_rows::value::KeyedRows;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::shared_index::value::SharedIndex;
use crate::domain::physical_plan::types::{
    IndexIdentity, IndexKey, IndexPlace, KeyedIndex, KeyedIndexes, NodeReaders, PlanNodeId,
    ProbeIndex,
};

/// Shares one `hash_index` between the probes of one right source and key: a planning step, not a rewrite, since an unshared
/// plan answers the same rows. A probe that filters its right rows indexes rows of its own and keeps its own index, as does an
/// ordered probe, which turns each key's first row into its extremum, and a probe alone on its right source and key; a probe
/// that walks the tree has no index to share. Run after the rewrites, which change a probe's right rows, key, mode and index.
pub(crate) fn shared_indexes(nodes: &mut [PhysicalNode], indexes: &mut Vec<KeyedIndex>) {
    let mut keyed = KeyedIndexes::default();
    for (node, id) in nodes.iter().zip((0..).map(PlanNodeId)) {
        if let Lowering::Probe {
            right,
            probe,
            index: ProbeIndex::Own,
            ..
        } = &node.lowering
            && let ProbeKey::Columns { right: on, .. } = &probe.key
            && probe.filter.comparisons.is_empty()
            && !matches!(probe.mode, ProbeMode::Ordered(_))
        {
            let _: IndexPlace = read_by(
                &mut keyed,
                IndexIdentity {
                    source: *right,
                    key: IndexKey::Cells(on.clone()),
                },
                id,
            );
        }
    }
    let keyed = keyed.indexes;
    for index in keyed.into_iter().filter(|index| index.readers.len() > 1) {
        let at = IndexPlace(indexes.len());
        for reader in &index.readers {
            if let Some(Lowering::Probe { index: shared, .. }) = nodes
                .get_mut(usize::try_from(reader.0).unwrap_or(usize::MAX))
                .map(|node| &mut node.lowering)
            {
                *shared = ProbeIndex::Shared(at);
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
        shared.indexes = plan
            .indexes
            .iter()
            .map(|_| SharedIndex::default())
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
            &mut buffers.indexes,
        );
    }
}

/// A probe of shared index `at` is done, whatever it answered: after the last of the file the index gives its rows to the
/// worker's index pool.
pub(crate) fn closed(buffers: &mut FileBuffers, at: IndexPlace) {
    if let Some(shared) = buffers.shared.indexes.get_mut(at.0) {
        shared.remaining = NodeReaders(shared.remaining.0.saturating_sub(1));
        if shared.remaining == NodeReaders(0) {
            shared.given_back(&mut buffers.indexes);
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
            Some(shared) if !shared.rows.next.is_empty() => IndexBuild::Built,
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
    own: &'b mut KeyedRows,
    index: ProbeIndex,
) -> Result<&'b mut KeyedRows, PortableOperatorFault> {
    match index {
        ProbeIndex::Own | ProbeIndex::Tree => Ok(own),
        ProbeIndex::Shared(at) => shared
            .get_mut(at.0)
            .map(|shared| &mut shared.rows)
            .ok_or(PortableOperatorFault::WrongOperation),
    }
}
