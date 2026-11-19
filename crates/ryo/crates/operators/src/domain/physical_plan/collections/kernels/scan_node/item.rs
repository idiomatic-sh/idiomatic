//! The `scan_node` item of the `kernels` collection: an earlier node's materialized rows, taken out of the buffers while its
//! reader reads them and given back after, with the fault it raised after its rows.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::kept_room::constants::POOL_TABLES;
use crate::domain::kept_room::types::PoolSize;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{NodeFed, NodeReaders, PlanNodeId, TakenRows};
#[cfg(test)]
use crate::domain::physical_plan::types::{RowSource, lowering::value::Lowering};
use ryo_values::domain::rows::types::Table;

/// `scan_node`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ScanNodeItem;

#[cfg(test)]
impl KernelsItem for ScanNodeItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = lowering
            .sources()
            .any(|source| matches!(source, RowSource::Node(_)));
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Takes `read`'s rows out of the buffers for its reader.
///
/// # Errors
///
/// `NotRun` when the node holds no rows.
pub(crate) fn take(
    buffers: &mut FileBuffers,
    read: PlanNodeId,
) -> Result<TakenRows, PortableOperatorFault> {
    let place = usize::try_from(read.0).unwrap_or(usize::MAX);
    let rows = buffers
        .nodes
        .get_mut(place)
        .and_then(Option::take)
        .ok_or(PortableOperatorFault::NotRun)?;
    let pending = buffers.pending.get_mut(place).and_then(Option::take);
    Ok(TakenRows { rows, pending })
}

/// Gives a read input back after `readings` of its readers are done with it; when they were its last readers and no engine
/// use of it remains, its rows go instead, their table to the worker's pool.
pub(crate) fn give_back(
    buffers: &mut FileBuffers,
    read: PlanNodeId,
    taken: TakenRows,
    readings: NodeReaders,
) {
    let place = usize::try_from(read.0).unwrap_or(usize::MAX);
    let left = buffers.remaining.get_mut(place).map_or(0, |left| {
        left.0 = left.0.saturating_sub(readings.0);
        left.0
    });
    let engine = buffers.engine_left.get(place).copied().unwrap_or_default();
    if left > 0 || engine.0 > 0 {
        if let Some(held) = buffers.nodes.get_mut(place) {
            *held = Some(taken.rows);
        }
        if let Some(pending) = buffers.pending.get_mut(place) {
            *pending = taken.pending;
        }
    } else {
        #[cfg(test)]
        buffers.meter.dropped(taken.rows.count);
        buffers.pool.pooled(taken.rows, PoolSize(POOL_TABLES));
    }
}

/// Holds a node's rows and pending fault, with all its readers and engine uses still to come.
pub(crate) fn hold(
    plan: &PhysicalPlan,
    buffers: &mut FileBuffers,
    node: PlanNodeId,
    rows: Table,
    pending: Option<PortableOperatorFault>,
) {
    let place = usize::try_from(node.0).unwrap_or(usize::MAX);
    // What the node was fed is set before its rows are held, in the span of the request that made it.
    crate::domain::physical_plan::service::run::counting::service::counted(
        buffers,
        node,
        crate::domain::physical_plan::types::node_counters::value::MadeIn::OpenSpan,
        rows.count,
    );
    // Spike (ryo-fmx3): what the open file knows of the node, kept past its release.
    let stamp = buffers.generation.wrapping_add(1);
    let empty = rows.count.0 == 0 && pending.is_none();
    // A node that ran and held no row emptied itself; one only marked empty before keeps who emptied it.
    if empty
        && buffers.known_empty.get(place) != Some(&stamp)
        && let Some(origin) = buffers.origin.get_mut(place)
    {
        *origin = node.0;
    }
    if let Some(known) = buffers.known_empty.get_mut(place) {
        *known = if empty { stamp } else { 0 };
    }
    if let Some(known) = buffers.known_full.get_mut(place) {
        *known = if empty { 0 } else { stamp };
    }
    if let Some(held) = buffers.nodes.get_mut(place) {
        #[cfg(test)]
        buffers.meter.formed(rows.count);
        *held = Some(rows);
        buffers.touched.touched(node);
    }
    if let Some(slot) = buffers.pending.get_mut(place) {
        *slot = pending;
    }
    // The readers and engine uses the file was done with before it made the node are not waited for.
    if let Some(physical) = plan.nodes.get(place) {
        let readers = buffers.done.readers_left(node, physical.readers);
        let engine = buffers.done.engine_left(node, physical.engine);
        if let Some(left) = buffers.remaining.get_mut(place) {
            *left = readers;
        }
        if let Some(left) = buffers.engine_left.get_mut(place) {
            *left = engine;
        }
    }
}

/// `node` was fed `fed` in the open file: what its kernels read from each input, kept until its release.
pub(crate) fn fed_with(buffers: &mut FileBuffers, node: PlanNodeId, fed: NodeFed) {
    if let Some(slot) = buffers
        .fed
        .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
    {
        *slot = fed;
        buffers.touched.touched(node);
    }
}
