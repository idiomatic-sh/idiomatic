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
        materialize(plan, run, node, buffers)?;
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
    let count = plan.node_count().0;
    if buffers.nodes.len() < count {
        buffers.nodes.resize_with(count, || None);
        buffers.pending.resize_with(count, || None);
        buffers.fed.resize(count, NodeFed::default());
        buffers.remaining.resize(count, NodeReaders(0));
        buffers.engine_left.resize(count, EngineUses(0));
        buffers.decisions.resize_with(count, NodeDecisions::default);
    }
    sized_shared(plan, buffers);
}

/// Runs an earlier node `source` reads when it holds no rows.
pub(super) fn ready(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    source: RowSource,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    match source {
        RowSource::Node(read) if buffers.rows(read) == NodeRows::Absent => {
            materialize(plan, run, read, buffers)
        }
        RowSource::Node(_)
        | RowSource::Facts(_)
        | RowSource::Tree(_)
        | RowSource::Frontier(_)
        | RowSource::Observation(_) => Ok(()),
    }
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
    let made = if let Some(table) = super::entity_queries::service::answer(plan, node, buffers) {
        sized(plan, buffers);
        hold(plan, buffers, node, table, None);
        Ok(())
    } else {
        kernels_of(plan, run, node, buffers)
    };
    timed_to(buffers, span, node);
    made
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
    let mut sources = lowering.sources();
    let first = sources
        .next()
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let second = sources.next();
    let third = sources.next();
    ready(plan, run, first, buffers)?;
    if let Some(second) = second {
        ready(plan, run, second, buffers)?;
    }
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
            kernels(lowering, shared, node, inputs, buffers, &mut output)
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
