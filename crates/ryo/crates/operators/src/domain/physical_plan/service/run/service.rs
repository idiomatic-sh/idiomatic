//! The runner: a node's kernels over one file's rows. A node keeps the rows it made before its first fault and raises the fault
//! after them; its reader reads every row before an input's fault and raises that fault only then, unless one of those rows
//! faults first, so the earliest row's fault wins whatever the pipeline (D15).
use super::super::kernels::service::kernels;
use super::types::{KernelInputs, Reading};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::lookup::item::lookups;
use crate::domain::physical_plan::collections::kernels::ordered_walk::item::interned;
use crate::domain::physical_plan::collections::kernels::scan_bound::item::scanned;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::count_of;
use crate::domain::physical_plan::collections::kernels::scan_node::item::fed_with;
use crate::domain::physical_plan::collections::kernels::scan_node::item::hold;
use crate::domain::physical_plan::service::indexes::service::sized_shared;
use crate::domain::physical_plan::service::run::counting::service::{timed, timed_to};
use crate::domain::physical_plan::service::run::reading::service::{
    pending_of, reading, returned, rows_in,
};
use crate::domain::physical_plan::types::file_buffers::value::{FileBuffers, NodeDecisions};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    EngineUses, KernelRun, NodeFed, NodeReaders, NodeRows, PlanNodeId, RowSource,
};
use ryo_values::domain::rows::types::RowCount;

/// Makes `node`'s rows for the open file in `buffers`, and answers the fault it raised after them.
///
/// # Errors
///
/// The node's first fault in the interpreter's order: a row's fault at its first failing stage, the earliest row first; a
/// refused relation; a node of a kind this stage does not lower.
pub(crate) fn run(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    node: PlanNodeId,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    // A node held in this file was made by a build that answered it with its index's other lookups, or by an earlier
    // request for it: its rows stand until the engine releases them at its last consumer.
    if buffers.rows(node) == NodeRows::Absent {
        if known(buffers, node.0) != Known::Empty {
            materialize(plan, run, node, buffers)?;
        }
        // Spike (ryo-fmx3): the engine reads the node's rows, so a node only marked empty is given its table now.
        installed(plan, buffers, node);
    }
    match buffers
        .pending
        .get(usize::try_from(node.0).unwrap_or(usize::MAX))
    {
        Some(Some(fault)) => Err(fault.clone()),
        Some(None) | None => Ok(()),
    }
}

/// Grows the per-node buffers to the plan's nodes, keeping what they hold.
fn sized(plan: &PhysicalPlan, buffers: &mut FileBuffers) {
    // Spike (ryo-fmx3): the buffers already grown for this plan fit it.
    let key = (
        std::ptr::from_ref(plan) as usize,
        plan.nodes.len(),
        plan.indexes.len(),
    );
    if buffers.sized_for == key {
        return;
    }
    let count = plan.node_count().0;
    if buffers.nodes.len() < count {
        buffers.nodes.resize_with(count, || None);
        buffers.pending.resize_with(count, || None);
        buffers.fed.resize(count, NodeFed::default());
        buffers.remaining.resize(count, NodeReaders(0));
        buffers.engine_left.resize(count, EngineUses(0));
        buffers.decisions.resize_with(count, NodeDecisions::default);
    }
    if buffers.known_empty.len() < count {
        buffers.known_empty.resize(count, 0);
        buffers.known_full.resize(count, 0);
    }
    sized_shared(plan, buffers);
    buffers.sized_for = key;
}

/// Spike (ryo-fmx3): whether evaluation is demand-driven, set once from `RYO_LAZY`: a node whose driving input holds no row
/// answers no row without readying its other inputs.
pub(crate) fn lazy() -> bool {
    crate::spike::lazy()
}

/// Spike (ryo-fmx3): whether `source` is a node the open file made with no row and no pending fault.
pub(crate) fn held_empty(buffers: &FileBuffers, source: RowSource) -> bool {
    let RowSource::Node(read) = source else {
        return false;
    };
    known(buffers, read.0) == Known::Empty
}

/// Spike (ryo-fmx3): what the open file knows of a node's rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Known {
    /// Not made in this file.
    Unmade,
    /// Made with no row and no pending fault.
    Empty,
    /// Made holding a row or a pending fault.
    Full,
}

/// Spike (ryo-fmx3): what the open file knows of node `node`.
pub(crate) fn known(buffers: &FileBuffers, node: u32) -> Known {
    let place = usize::try_from(node).unwrap_or(usize::MAX);
    let stamp = buffers.generation.wrapping_add(1);
    if buffers.known_empty.get(place) == Some(&stamp) {
        Known::Empty
    } else if buffers.known_full.get(place) == Some(&stamp) {
        Known::Full
    } else {
        Known::Unmade
    }
}

/// Spike (ryo-fmx3): holds `node` with no row, in its planned layout, in the room of `table`.
pub(crate) fn hold_empty_in(
    plan: &PhysicalPlan,
    buffers: &mut FileBuffers,
    node: PlanNodeId,
    mut table: ryo_values::domain::rows::types::Table,
) {
    sized(plan, buffers);
    table.cells.clear();
    table.count = RowCount(0);
    table.kinds.clear();
    if let Some(Some(kinds)) = plan
        .layouts
        .get(usize::try_from(node.0).unwrap_or(usize::MAX))
    {
        table.kinds.extend_from_slice(kinds);
    }
    hold(plan, buffers, node, table, None);
}

/// Runs an earlier node `source` reads when it holds no rows, and holds its table for the reader.
pub(super) fn ready(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    source: RowSource,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    settled(plan, run, source, buffers)?;
    if let RowSource::Node(read) = source {
        installed(plan, buffers, read);
    }
    Ok(())
}

/// Spike (ryo-fmx3): runs an earlier node `source` reads when the open file has not made it; a node found empty is only
/// marked, its table held when a reader asks for it (`installed`).
pub(crate) fn settled(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    source: RowSource,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    match source {
        RowSource::Node(read)
            if buffers.rows(read) == NodeRows::Absent && known(buffers, read.0) != Known::Empty =>
        {
            materialize(plan, run, read, buffers)
        }
        RowSource::Node(_)
        | RowSource::Facts(_)
        | RowSource::Tree(_)
        | RowSource::Frontier(_)
        | RowSource::Observation(_) => Ok(()),
    }
}

/// Spike (ryo-fmx3): holds the empty table of a node the open file only marked empty.
pub(crate) fn installed(plan: &PhysicalPlan, buffers: &mut FileBuffers, node: PlanNodeId) {
    if buffers.rows(node) == NodeRows::Absent && known(buffers, node.0) == Known::Empty {
        let table = buffers.pool.fresh();
        hold_empty_in(plan, buffers, node, table);
    }
}

/// Spike (ryo-fmx3): marks `node` empty in the open file without holding a table.
pub(crate) fn marked_empty(plan: &PhysicalPlan, buffers: &mut FileBuffers, node: PlanNodeId) {
    sized(plan, buffers);
    let place = usize::try_from(node.0).unwrap_or(usize::MAX);
    let stamp = buffers.generation.wrapping_add(1);
    if let Some(known) = buffers.known_empty.get_mut(place) {
        *known = stamp;
    }
    if let Some(known) = buffers.known_full.get_mut(place) {
        *known = 0;
    }
    crate::spike::skip(node.0);
}

/// Runs `node`'s kernels and holds its rows and pending fault: a streaming stage through the chain of streamed stages that
/// ends at it; a lookup's build answers every lookup of its index. When counting is on, the time it takes, less the time
/// of the nodes it makes first, is `node`'s.
fn materialize(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    node: PlanNodeId,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    let span = timed(buffers);
    if crate::spike::on() {
        crate::spike::enter(node.0);
    }
    let made = if guarded_empty(plan, run, node, buffers) {
        marked_empty(plan, buffers, node);
        Ok(())
    } else if let Some(table) = super::entity_queries::service::answer(plan, node, buffers) {
        sized(plan, buffers);
        hold(plan, buffers, node, table, None);
        Ok(())
    } else {
        kernels_of(plan, run, node, buffers)
    };
    if crate::spike::on() {
        crate::spike::leave(match buffers.rows(node) {
            NodeRows::Materialized(table) => table.count.0,
            NodeRows::Absent => 0,
        });
    }
    timed_to(buffers, span, node);
    made
}

/// Spike (ryo-fmx3): whether one of the cheap leaves `node` cannot hold a row without holds none in the open file; each
/// leaf not yet made in this file is made first. A leaf that faults guards nothing.
fn guarded_empty(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    node: PlanNodeId,
    buffers: &mut FileBuffers,
) -> bool {
    if !lazy() || crate::spike::guards_off() {
        return false;
    }
    let Some(guards) = plan
        .guards
        .get(usize::try_from(node.0).unwrap_or(usize::MAX))
    else {
        return false;
    };
    sized(plan, buffers);
    // First what the file already knows of the leaves, which costs nothing; then the leaves still to make.
    let stamp = buffers.generation.wrapping_add(1);
    if guards.iter().any(|guard| {
        buffers
            .known_empty
            .get(usize::try_from(*guard).unwrap_or(usize::MAX))
            == Some(&stamp)
    }) {
        return true;
    }
    for guard in guards.iter() {
        if known(buffers, *guard) != Known::Unmade {
            continue;
        }
        if materialize(plan, run, PlanNodeId(*guard), buffers).is_err() {
            continue;
        }
        if known(buffers, *guard) == Known::Empty {
            return true;
        }
    }
    false
}

/// What `materialize` runs for `node`.
fn kernels_of(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    node: PlanNodeId,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    sized(plan, buffers);
    let place = usize::try_from(node.0).unwrap_or(usize::MAX);
    let lowering = &plan
        .nodes
        .get(place)
        .ok_or(PortableOperatorFault::WrongOperation)?
        .lowering;
    if let Lowering::Filter { .. }
    | Lowering::Project { .. }
    | Lowering::Extend { .. }
    | Lowering::TextMap { .. }
    | Lowering::Probe { .. } = lowering
        && !(lazy() && crate::spike::held_only())
    {
        return super::pipeline::service::streamed(plan, run, node, buffers, ready);
    }
    made(plan, run, node, lowering, buffers)
}

/// Runs the kernels of `node`, lowered to `lowering`, over its inputs, and holds its rows and pending fault.
pub(super) fn made(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    node: PlanNodeId,
    lowering: &Lowering,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    match lowering {
        Lowering::Lookup { index, .. } => return lookups(plan, run, *index, buffers),
        Lowering::ScanBound { scan } => {
            let rows = scanned(scan, run)?;
            // What the scan was fed: the bound rows it reads.
            fed_with(
                buffers,
                node,
                NodeFed([rows.count, RowCount(0), RowCount(0)]),
            );
            hold(plan, buffers, node, rows, None);
            return Ok(());
        }
        Lowering::Unlowered => return Err(PortableOperatorFault::NotRun),
        Lowering::Filter { .. }
        | Lowering::Project { .. }
        | Lowering::Extend { .. }
        | Lowering::Group { .. }
        | Lowering::Probe { .. }
        | Lowering::Distinct { .. }
        | Lowering::TextMap { .. }
        | Lowering::OrderedMatch { .. }
        | Lowering::Render { .. } => {}
        Lowering::Reach { .. }
        | Lowering::SeededReach { .. }
        | Lowering::Axis { .. }
        | Lowering::OrderedWalk { .. }
        | Lowering::Worklist { .. }
        | Lowering::Components { .. } => {}
    }
    let [first, second, third] = lowering.source_slots();
    let first = first.ok_or(PortableOperatorFault::WrongOperation)?;
    settled(plan, run, first, buffers)?;
    if lazy()
        && matches!(
            lowering,
            Lowering::SeededReach { .. }
                | Lowering::Axis { .. }
                | Lowering::OrderedWalk { .. }
                | Lowering::Worklist { .. }
                | Lowering::OrderedMatch { .. }
        )
        && held_empty(buffers, first)
    {
        marked_empty(plan, buffers, node);
        return Ok(());
    }
    if let Some(second) = second {
        settled(plan, run, second, buffers)?;
        if lazy()
            && matches!(lowering, Lowering::Distinct { .. })
            && held_empty(buffers, first)
            && held_empty(buffers, second)
        {
            marked_empty(plan, buffers, node);
            return Ok(());
        }
        ready(plan, run, second, buffers)?;
    }
    ready(plan, run, first, buffers)?;
    if let Some(third) = third {
        ready(plan, run, third, buffers)?;
    }
    if let Lowering::Render { .. } = lowering {
        return super::minted::service::minted(plan, run, node, lowering, first, buffers);
    }
    let shared: &KernelRun<'_> = run;
    // A node read by several inputs is taken once and read by each of them, and gives back one read per input.
    let sources = [Some(first), second, third];
    let mut slots: [Option<Reading<'_>>; 3] = [None, None, None];
    let mut reads = [NodeReaders(0); 3];
    let mut slot_of = [0, 1, 2];
    for (at, source) in sources.iter().enumerate() {
        let Some(source) = *source else {
            continue;
        };
        let earlier = (0..at).find(|before| {
            matches!(source, RowSource::Node(_))
                && sources.get(*before).copied().flatten() == Some(source)
        });
        if let Some(before) = earlier {
            let slot = slot_of.get(before).copied().unwrap_or(before);
            if let Some(place) = slot_of.get_mut(at) {
                *place = slot;
            }
            if let Some(count) = reads.get_mut(slot) {
                count.0 += 1;
            }
            continue;
        }
        match reading(shared, node, source, buffers) {
            Ok(read) => {
                if let (Some(slot), Some(count)) = (slots.get_mut(at), reads.get_mut(at)) {
                    *slot = Some(read);
                    *count = NodeReaders(1);
                }
            }
            Err(fault) => {
                for (slot, count) in slots.into_iter().zip(reads) {
                    if let Some(read) = slot {
                        returned(buffers, read, count);
                    }
                }
                return Err(fault);
            }
        }
    }
    let read_at: [Option<&Reading<'_>>; 3] = std::array::from_fn(|at| {
        let _source = sources.get(at).copied().flatten()?;
        slots.get(slot_of.get(at).copied()?)?.as_ref()
    });
    // What the node was fed: each input's rows, a node read by two inputs in both.
    fed_with(
        buffers,
        node,
        NodeFed(
            read_at.map(|read| read.map_or_else(Default::default, |read| count_of(rows_in(read)))),
        ),
    );
    let [first_read, second_read, third_read] = read_at;
    let left = first_read.ok_or(PortableOperatorFault::WrongOperation)?;
    let left_fault = pending_of(left);
    let right_fault = second_read.and_then(pending_of);
    let last_fault = third_read.and_then(pending_of);
    // A blocking kernel reads every row of an input before it answers one: an input's fault comes before any of its rows. A
    // union reads its right rows only after every left row.
    // A graph kernel reads every edge row before it walks: the edges' fault comes first, then an axis's domain's, then an
    // ordered walk's roots' (sorted before use).
    let blocked = match lowering {
        Lowering::SeededReach { .. } => right_fault.clone(),
        Lowering::Group { .. }
        | Lowering::Render { .. }
        | Lowering::Reach { .. }
        | Lowering::Components { .. } => left_fault.clone(),
        Lowering::OrderedMatch { .. } => left_fault.clone().or_else(|| right_fault.clone()),
        Lowering::Axis { .. } => right_fault.clone().or_else(|| last_fault.clone()),
        Lowering::OrderedWalk { .. } => right_fault.clone().or_else(|| left_fault.clone()),
        Lowering::Worklist { .. } => right_fault
            .clone()
            .or_else(|| last_fault.clone())
            .or_else(|| left_fault.clone()),
        Lowering::Filter { .. }
        | Lowering::Project { .. }
        | Lowering::Extend { .. }
        | Lowering::Probe { .. }
        | Lowering::Distinct { .. }
        | Lowering::TextMap { .. }
        | Lowering::Lookup { .. }
        | Lowering::ScanBound { .. }
        | Lowering::Unlowered => None,
    };
    let walked = matches!(lowering, Lowering::OrderedWalk { .. }) && blocked.is_none();
    // The node's table: one the worker's pool kept, its room reused.
    let mut output = buffers.pool.fresh();
    let made = match blocked {
        Some(fault) => Ok(Some(fault)),
        None => {
            let right_rows =
                if left_fault.is_some() && matches!(lowering, Lowering::Distinct { .. }) {
                    None
                } else {
                    second_read.map(rows_in)
                };
            let inputs = KernelInputs {
                input: rows_in(left),
                other: right_rows,
                third: third_read.map(rows_in),
            };
            let answered = kernels(lowering, shared, node, inputs, buffers, &mut output);
            // Spike (ryo-fmx3): which held kernel faulted, for the no-fusion mode's diagnosis.
            if crate::spike::held_only()
                && !matches!(answered, Ok(None))
                && std::env::var_os("RYO_HELD_DEBUG").is_some()
            {
                eprintln!(
                    "held fault at node {}: {:?}; input kinds {:?}; other kinds {:?}; lowering {:.400?}",
                    node.0,
                    answered,
                    crate::domain::physical_plan::collections::kernels::scan_facts::item::kinds_of(inputs.input),
                    inputs.other.map(crate::domain::physical_plan::collections::kernels::scan_facts::item::kinds_of),
                    lowering
                );
            }
            answered
        }
    };
    for (slot, count) in slots.into_iter().zip(reads) {
        if let Some(read) = slot {
            returned(buffers, read, count);
        }
    }
    // Main's ordered walk interned the texts its keys met that the file did not hold yet, for the rest of the file: the
    // kernel numbered them in that order, and the file's symbols now hold them so the next walk finds them.
    if walked {
        interned(&buffers.walk.reached, run.resources);
    }
    let fault = made?;
    hold(
        plan,
        buffers,
        node,
        output,
        fault.or(left_fault).or(right_fault).or(last_fault),
    );
    Ok(())
}
