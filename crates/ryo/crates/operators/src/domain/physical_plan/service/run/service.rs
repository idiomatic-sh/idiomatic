//! The runner: a node's kernels over one file's rows. A node keeps the rows it made before its first fault and raises the fault
//! after them; its reader reads every row before an input's fault and raises that fault only then, unless one of those rows
//! faults first, so the earliest row's fault wins whatever the pipeline (D15).
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::service::filter::service::{filter, keep, kept_kinds, projected};
use crate::domain::physical_plan::service::index::service::{
    decode, facts, hold, lookups, rows_of,
};
use crate::domain::physical_plan::types::file_buffers::value::{FileBuffers, NodeDecisions};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    KernelRun, Lowering, NodeReaders, PlanNodeId, Reading, Retention, RowSource, TakenRows,
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

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
    let held = buffers
        .nodes
        .get(usize::try_from(node.0).unwrap_or(usize::MAX))
        .is_some_and(Option::is_some);
    if !held {
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

/// Readies what `source` reads: an earlier node is run first when it holds no rows.
fn reading<'a>(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'a>,
    source: RowSource,
    buffers: &mut FileBuffers,
) -> Result<Reading<'a>, PortableOperatorFault> {
    match source {
        RowSource::Facts(relation) => facts(run.inputs, relation).map(Reading::Facts),
        RowSource::Node(read) => {
            let place = usize::try_from(read.0).unwrap_or(usize::MAX);
            if buffers.nodes.get(place).is_none_or(Option::is_none) {
                materialize(plan, run, read, buffers)?;
            }
            take(buffers, read).map(|taken| Reading::Node(read, taken))
        }
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
    let physical = plan
        .nodes
        .get(place)
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let (source, mut output) = match &physical.lowering {
        Lowering::Lookup { index, .. } => return lookups(plan, run, *index, buffers),
        Lowering::Unlowered => return Err(PortableOperatorFault::NotRun),
        Lowering::Filter { source, .. } | Lowering::Project { source, .. } => {
            (*source, Table::default())
        }
    };
    let input = reading(plan, run, source, buffers)?;
    let (kinds, rows): (&[_], RowCount) = match &input {
        Reading::Facts(view) => (view.kinds, rows_of(view)),
        Reading::Node(_, taken) => (&taken.rows.kinds, taken.rows.count),
    };
    let mut fault = None;
    match &physical.lowering {
        Lowering::Filter { select, .. } => {
            output.kinds = kept_kinds(kinds, &select.kept)?;
            let texts = run.resources.texts();
            let decisions = buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            let kernel = filter(select, kinds, texts, decisions);
            for row in 0..rows.0 {
                let cells = row_of(&input, RowIndex(row), &mut buffers.row);
                if let Err(error) = keep(
                    kernel,
                    run.matcher,
                    cells,
                    decisions,
                    &mut buffers.written,
                    &mut output,
                ) {
                    fault = Some(error);
                    break;
                }
            }
        }
        Lowering::Project {
            columns, distinct, ..
        } => {
            output.kinds = kept_kinds(kinds, columns)?;
            let mut seen = hashbrown::HashTable::new();
            for row in 0..rows.0 {
                let cells = row_of(&input, RowIndex(row), &mut buffers.row);
                if let Err(error) = projected(cells, columns, *distinct, &mut seen, &mut output) {
                    fault = Some(error);
                    break;
                }
            }
        }
        Lowering::Lookup { .. } | Lowering::Unlowered => {}
    }
    if let Reading::Node(read, taken) = input {
        let pending = taken.pending.clone();
        give_back(plan, buffers, read, taken);
        fault = fault.or(pending);
    }
    hold(plan, buffers, node, output, fault);
    Ok(())
}

/// Row `row` of what a kernel reads: a node's cells in place, a relation's decoded into `scratch`.
fn row_of<'r>(input: &'r Reading<'_>, row: RowIndex, scratch: &'r mut Vec<Value>) -> &'r [Value] {
    match input {
        Reading::Facts(view) => {
            decode(view, row, scratch);
            scratch
        }
        Reading::Node(_, taken) => {
            let width = taken.rows.kinds.len().max(1);
            taken
                .rows
                .cells
                .get(row.0 * width..row.0 * width + width)
                .unwrap_or_default()
        }
    }
}

/// Takes `read`'s rows out of the buffers for its reader.
fn take(buffers: &mut FileBuffers, read: PlanNodeId) -> Result<TakenRows, PortableOperatorFault> {
    let place = usize::try_from(read.0).unwrap_or(usize::MAX);
    let rows = buffers
        .nodes
        .get_mut(place)
        .and_then(Option::take)
        .ok_or(PortableOperatorFault::NotRun)?;
    let pending = buffers.pending.get_mut(place).and_then(Option::take);
    Ok(TakenRows { rows, pending })
}

/// Gives a read input back after its reader is done; it is released instead when that was its last reader and no consumer
/// keeps it.
fn give_back(plan: &PhysicalPlan, buffers: &mut FileBuffers, read: PlanNodeId, taken: TakenRows) {
    let place = usize::try_from(read.0).unwrap_or(usize::MAX);
    let left = buffers.remaining.get_mut(place).map_or(0, |left| {
        left.0 = left.0.saturating_sub(1);
        left.0
    });
    let retained = plan
        .nodes
        .get(place)
        .is_some_and(|node| node.retained == Retention::Retained);
    if left > 0 || retained {
        if let Some(held) = buffers.nodes.get_mut(place) {
            *held = Some(taken.rows);
        }
        if let Some(pending) = buffers.pending.get_mut(place) {
            *pending = taken.pending;
        }
    }
}
