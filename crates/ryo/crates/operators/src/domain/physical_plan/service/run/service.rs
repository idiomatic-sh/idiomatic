//! The runner: a node's kernels over one file's rows. A node keeps the rows it made before its first fault and raises the fault
//! after them; its reader reads every row before an input's fault and raises that fault only then, unless one of those rows
//! faults first, so the earliest row's fault wins whatever the pipeline (D15).
use super::super::kernels::service::kernels;
use super::types::{KernelInputs, Reading};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::extend::item::resolved;
use crate::domain::physical_plan::collections::kernels::lookup::item::lookups;
use crate::domain::physical_plan::collections::kernels::ordered_walk::item::interned;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::facts;
use crate::domain::physical_plan::collections::kernels::scan_node::item::{give_back, hold, take};
use crate::domain::physical_plan::collections::kernels::scan_roots::item::roots;
use crate::domain::physical_plan::types::file_buffers::value::{FileBuffers, NodeDecisions};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    InputRows, KernelRun, NodeReaders, NodeRows, PlanNodeId, RowSource,
};
use ryo_values::domain::rows::types::Table;

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
    let count = plan.nodes.len();
    if buffers.nodes.len() < count {
        buffers.nodes.resize_with(count, || None);
        buffers.pending.resize_with(count, || None);
        buffers.remaining.resize(count, NodeReaders(0));
        buffers.decisions.resize_with(count, NodeDecisions::default);
    }
}

/// Runs an earlier node `source` reads when it holds no rows.
fn ready(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    source: RowSource,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    match source {
        RowSource::Node(read) if buffers.rows(read) == NodeRows::Absent => {
            materialize(plan, run, read, buffers)
        }
        RowSource::Node(_) | RowSource::Facts(_) | RowSource::Tree(_) => Ok(()),
    }
}

/// What `source` reads, taken for its reader `node`: a relation's rows lent in place, a tree relation's rows at the tree nodes
/// the engine names for the reader, or a ready node's rows.
fn reading<'s>(
    run: &'s KernelRun<'_>,
    node: PlanNodeId,
    source: RowSource,
    buffers: &mut FileBuffers,
) -> Result<Reading<'s>, PortableOperatorFault> {
    match source {
        RowSource::Facts(relation) => facts(run.inputs, relation).map(Reading::Facts),
        RowSource::Tree(relation) => Ok(Reading::Tree(roots(
            run.inputs,
            run.resources.tree(),
            node,
            relation,
        ))),
        RowSource::Node(read) => take(buffers, read).map(|taken| Reading::Node(read, taken)),
    }
}

/// The rows of a reading, read in place.
fn rows_in<'r>(reading: &'r Reading<'_>) -> InputRows<'r> {
    match reading {
        Reading::Facts(view) => InputRows::Facts(view),
        Reading::Tree(tree) => InputRows::Tree(tree),
        Reading::Node(_, taken) => InputRows::Table(&taken.rows),
    }
}

/// Gives a reading back after `readings` of its node's readers.
fn returned(
    plan: &PhysicalPlan,
    buffers: &mut FileBuffers,
    reading: Reading<'_>,
    readings: NodeReaders,
) {
    if let Reading::Node(read, taken) = reading {
        give_back(plan, buffers, read, taken, readings);
    }
}

/// Runs `node`'s kernels and holds its rows and pending fault; a lookup's build answers every lookup of its index.
fn materialize(
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
    match lowering {
        Lowering::Lookup { index, .. } => return lookups(plan, run, *index, buffers),
        Lowering::Unlowered => return Err(PortableOperatorFault::NotRun),
        Lowering::Filter { .. }
        | Lowering::Project { .. }
        | Lowering::Extend { .. }
        | Lowering::Group { .. }
        | Lowering::Probe { .. }
        | Lowering::Distinct { .. }
        | Lowering::Reach { .. }
        | Lowering::SeededReach { .. }
        | Lowering::Axis { .. }
        | Lowering::OrderedWalk { .. }
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
    // An extension's values are made before its rows are read: making a name changes the file's names, and reading reads them.
    let resolution = match lowering {
        Lowering::Extend { extension, .. } => resolved(
            &extension.values,
            &extension.bindings,
            run.resources,
            &mut buffers.values,
        )
        .err(),
        Lowering::Filter { .. }
        | Lowering::Project { .. }
        | Lowering::Group { .. }
        | Lowering::Probe { .. }
        | Lowering::Distinct { .. }
        | Lowering::Lookup { .. }
        | Lowering::Reach { .. }
        | Lowering::SeededReach { .. }
        | Lowering::Axis { .. }
        | Lowering::OrderedWalk { .. }
        | Lowering::Components { .. }
        | Lowering::Unlowered => None,
    };
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
                        returned(plan, buffers, read, count);
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
        Lowering::Probe { .. } | Lowering::SeededReach { .. } => right_fault.clone(),
        Lowering::Group { .. } | Lowering::Reach { .. } | Lowering::Components { .. } => {
            left_fault.clone()
        }
        Lowering::Axis { .. } => right_fault.clone().or_else(|| last_fault.clone()),
        Lowering::OrderedWalk { .. } => right_fault.clone().or_else(|| left_fault.clone()),
        Lowering::Filter { .. }
        | Lowering::Project { .. }
        | Lowering::Extend { .. }
        | Lowering::Distinct { .. }
        | Lowering::Lookup { .. }
        | Lowering::Unlowered => None,
    };
    let walked = matches!(lowering, Lowering::OrderedWalk { .. }) && blocked.is_none();
    let mut output = Table::default();
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
            kernels(
                lowering,
                shared,
                node,
                inputs,
                resolution,
                buffers,
                &mut output,
            )
        }
    };
    for (slot, count) in slots.into_iter().zip(reads) {
        if let Some(read) = slot {
            returned(plan, buffers, read, count);
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

/// The fault a read node raised after its rows.
fn pending_of(reading: &Reading<'_>) -> Option<PortableOperatorFault> {
    match reading {
        Reading::Facts(_) | Reading::Tree(_) => None,
        Reading::Node(_, taken) => taken.pending.clone(),
    }
}
