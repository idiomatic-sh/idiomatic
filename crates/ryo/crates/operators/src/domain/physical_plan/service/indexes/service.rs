//! The probe indexes a plan shares: the planning step that shares one `hash_index` between the probes of one right side and
//! key, decided once as the plan is built; and each shared index's life in a file, from its first probe to its last.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::ProbeKey;
use crate::domain::physical_plan::collections::kernels::probe::item::{IndexBuild, ProbeScratch};
use crate::domain::physical_plan::types::file_buffers::value::{FileBuffers, SharedIndex};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    IndexKey, IndexPlace, KeyedIndex, Lowering, NodeReaders, PhysicalNode, PlanNodeId, ProbeIndex,
};

/// Shares one `hash_index` between the probes of one right source and key: a planning step, not a rewrite, since an unshared
/// plan answers the same rows. A probe that filters its right rows indexes rows of its own and keeps its own index, as does a
/// probe alone on its right source and key.
pub(crate) fn shared_indexes(nodes: &mut [PhysicalNode], indexes: &mut Vec<KeyedIndex>) {
    let mut keyed: Vec<KeyedIndex> = Vec::new();
    for (node, id) in nodes.iter().zip((0..).map(PlanNodeId)) {
        if let Lowering::Probe { right, probe, .. } = &node.lowering
            && let ProbeKey::Columns { right: on, .. } = &probe.key
            && probe.filter.comparisons.is_empty()
        {
            let key = IndexKey::Cells(on.clone());
            match keyed
                .iter_mut()
                .find(|index| index.source == *right && index.key == key)
            {
                Some(index) => index.readers.push(id),
                None => keyed.push(KeyedIndex {
                    source: *right,
                    key,
                    readers: vec![id],
                }),
            }
        }
    }
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

/// One shared index for each index of `plan`, each emptied by the release of its last probe.
pub(crate) fn sized_shared(plan: &PhysicalPlan, buffers: &mut FileBuffers) {
    if buffers.shared.len() < plan.indexes.len() {
        buffers.shared = plan
            .indexes
            .iter()
            .map(|index| SharedIndex::new(index.readers.last().copied().unwrap_or(PlanNodeId(0))))
            .collect();
    }
}

/// A probe of shared index `at` runs: the first of the file counts every probe of it as still to come.
pub(crate) fn opened(plan: &PhysicalPlan, buffers: &mut FileBuffers, at: IndexPlace) {
    if let (Some(shared), Some(index)) = (buffers.shared.get_mut(at.0), plan.indexes.get(at.0))
        && shared.remaining == NodeReaders(0)
    {
        shared.remaining = NodeReaders(u32::try_from(index.readers.len()).unwrap_or(u32::MAX));
    }
}

/// A probe of shared index `at` is done, whatever it answered: after the last of the file the index is emptied.
pub(crate) fn closed(buffers: &mut FileBuffers, at: IndexPlace) {
    if let Some(shared) = buffers.shared.get_mut(at.0) {
        shared.remaining = NodeReaders(shared.remaining.0.saturating_sub(1));
        if shared.remaining == NodeReaders(0) {
            shared.emptied();
        }
    }
}

/// What probe `node` writes through: its own index, built by this run, or its shared index, built by the first probe
/// of the file that reads its rows.
///
/// # Errors
///
/// `WrongOperation` for a node or an index the buffers do not hold.
pub(crate) fn probe_scratch(
    buffers: &mut FileBuffers,
    index: ProbeIndex,
    node: PlanNodeId,
) -> Result<ProbeScratch<'_>, PortableOperatorFault> {
    let decisions = buffers
        .decisions
        .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let (index, build) = match index {
        ProbeIndex::Own => (&mut buffers.index, IndexBuild::Builds),
        ProbeIndex::Shared(at) => {
            let shared = buffers
                .shared
                .get_mut(at.0)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            // An index of no row is built again at no cost: an empty chain is an index not built in this file.
            let build = if shared.rows.next.is_empty() {
                IndexBuild::Builds
            } else {
                IndexBuild::Built
            };
            (&mut shared.rows, build)
        }
    };
    #[cfg(test)]
    if build == IndexBuild::Builds {
        buffers.builds.0 += 1;
    }
    Ok(ProbeScratch {
        index,
        build,
        row: &mut buffers.row,
        written: &mut buffers.written,
        repeats: &mut buffers.repeats,
        decisions,
    })
}
