//! The runner: a node's kernels over one file's rows. A node keeps the rows it made before its first fault and raises the fault
//! after them; its reader reads every row before an input's fault and raises that fault only then, unless one of those rows
//! faults first, so the earliest row's fault wins whatever the pipeline (D15).
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::distinct::item::counted;
use crate::domain::physical_plan::collections::kernels::extend::item::{extended, kind_of, resolved};
use crate::domain::physical_plan::collections::kernels::filter::item::{filter, keep, kept_kinds};
use crate::domain::physical_plan::collections::kernels::group::item::{GroupScratch, grouped};
use crate::domain::physical_plan::collections::kernels::lookup::item::lookups;
use crate::domain::physical_plan::collections::kernels::probe::item::{ProbeScratch, probed};
use crate::domain::physical_plan::collections::kernels::project::item::projected;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::{
    count_of, facts, kinds_of, row_of,
};
use crate::domain::physical_plan::collections::kernels::scan_roots::item::roots;
use crate::domain::physical_plan::collections::kernels::scan_node::item::{
    give_back, hold, sources_of, take,
};
use crate::domain::physical_plan::types::file_buffers::value::{FileBuffers, NodeDecisions};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    CellStart, InputRows, KernelInputs, KernelRun, Lowering, NodeReaders, NodeRows, PlanNodeId,
    Reading, RowSource,
};
use ryo_values::domain::rows::types::{RowIndex, Table};

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
        | Lowering::Distinct { .. } => {}
    }
    let mut sources = sources_of(lowering);
    let first = sources.next().ok_or(PortableOperatorFault::WrongOperation)?;
    let second = sources.next();
    ready(plan, run, first, buffers)?;
    if let Some(second) = second {
        ready(plan, run, second, buffers)?;
    }
    // An extension's values are made before its rows are read: making a name changes the file's names, and reading reads them.
    let resolution = match lowering {
        Lowering::Extend { extension, .. } => {
            resolved(&extension.values, &extension.bindings, run.resources, &mut buffers.values).err()
        }
        Lowering::Filter { .. }
        | Lowering::Project { .. }
        | Lowering::Group { .. }
        | Lowering::Probe { .. }
        | Lowering::Distinct { .. }
        | Lowering::Lookup { .. }
        | Lowering::Unlowered => None,
    };
    let run: &KernelRun<'_> = run;
    // One node read on both sides is taken once and read twice.
    let shared = second == Some(first) && matches!(first, RowSource::Node(_));
    let left = reading(run, node, first, buffers)?;
    let right = match second {
        Some(second) if !shared => match reading(run, node, second, buffers) {
            Ok(right) => Some(right),
            Err(fault) => {
                returned(plan, buffers, left, NodeReaders(1));
                return Err(fault);
            }
        },
        Some(_) | None => None,
    };
    let left_fault = pending_of(&left);
    let right_fault = match &right {
        Some(right) => pending_of(right),
        None if shared => left_fault.clone(),
        None => None,
    };
    // A blocking kernel reads every row of an input before it answers one: an input's fault comes before any of its rows. A
    // union reads its right rows only after every left row.
    let blocked = match lowering {
        Lowering::Probe { .. } => right_fault.clone(),
        Lowering::Group { .. } => left_fault.clone(),
        Lowering::Filter { .. }
        | Lowering::Project { .. }
        | Lowering::Extend { .. }
        | Lowering::Distinct { .. }
        | Lowering::Lookup { .. }
        | Lowering::Unlowered => None,
    };
    let mut output = Table::default();
    let made = match blocked {
        Some(fault) => Ok(Some(fault)),
        None => {
            let right_rows = match (&right, left_fault.is_some()) {
                (_, true) if matches!(lowering, Lowering::Distinct { .. }) => None,
                (Some(right), _) => Some(rows_in(right)),
                (None, _) => shared.then(|| rows_in(&left)),
            };
            let inputs = KernelInputs {
                input: rows_in(&left),
                other: right_rows,
            };
            kernels(lowering, run, node, inputs, resolution, buffers, &mut output)
        }
    };
    returned(plan, buffers, left, NodeReaders(if shared { 2 } else { 1 }));
    if let Some(right) = right {
        returned(plan, buffers, right, NodeReaders(1));
    }
    let fault = made?;
    hold(plan, buffers, node, output, fault.or(left_fault).or(right_fault));
    Ok(())
}

/// The fault a read node raised after its rows.
fn pending_of(reading: &Reading<'_>) -> Option<PortableOperatorFault> {
    match reading {
        Reading::Facts(_) | Reading::Tree(_) => None,
        Reading::Node(_, taken) => taken.pending.clone(),
    }
}

/// Makes a node's rows from its inputs' rows into `output`, and answers the fault of the first row that raised one.
///
/// # Errors
///
/// A fault of the node's shape: a column past its input's rows.
fn kernels(
    lowering: &Lowering,
    run: &KernelRun<'_>,
    node: PlanNodeId,
    KernelInputs { input, other }: KernelInputs<'_>,
    resolution: Option<PortableOperatorFault>,
    buffers: &mut FileBuffers,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let place = usize::try_from(node.0).unwrap_or(usize::MAX);
    let kinds = kinds_of(input);
    let rows = count_of(input).0;
    match lowering {
        Lowering::Filter { select, .. } => {
            output.kinds = kept_kinds(kinds, &select.kept)?;
            let texts = run.resources.texts();
            let decisions = buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            let kernel = filter(select, kinds, texts, decisions);
            for row in 0..rows {
                let cells = row_of(input, RowIndex(row), &mut buffers.row);
                if let Err(fault) = keep(kernel, run.matcher, cells, decisions, &mut buffers.written, output) {
                    return Ok(Some(fault));
                }
            }
        }
        Lowering::Project { project, .. } => {
            output.kinds = kept_kinds(kinds, &project.columns)?;
            buffers.seen.clear();
            for row in 0..rows {
                let cells = row_of(input, RowIndex(row), &mut buffers.row);
                if let Err(fault) = projected(cells, &project.columns, project.distinct, &mut buffers.seen, output) {
                    return Ok(Some(fault));
                }
            }
        }
        Lowering::Extend { .. } => {
            if let Some(fault) = resolution {
                return Ok(Some(fault));
            }
            output.kinds = kinds.iter().copied().chain(buffers.values.iter().map(|value| kind_of(*value))).collect();
            for row in 0..rows {
                let cells = row_of(input, RowIndex(row), &mut buffers.row);
                extended(cells, &buffers.values, &mut buffers.written, output);
            }
        }
        Lowering::Distinct { .. } => {
            output.kinds = kinds.to_vec();
            buffers.seen.clear();
            for source in std::iter::once(input).chain(other) {
                if kinds_of(source) != kinds {
                    return Err(PortableOperatorFault::ShapeMismatch);
                }
                for row in 0..count_of(source).0 {
                    let start = CellStart(output.cells.len());
                    output.cells.extend_from_slice(row_of(source, RowIndex(row), &mut buffers.row));
                    counted(output, start, ryo_values::domain::plan_parts::types::RepeatedRows::Dropped, &mut buffers.seen);
                }
            }
        }
        Lowering::Probe { probe, .. } => {
            let right = other.ok_or(PortableOperatorFault::WrongOperation)?;
            let decisions = buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            let scratch = ProbeScratch {
                index: &mut buffers.index,
                row: &mut buffers.row,
                written: &mut buffers.written,
                seen: &mut buffers.seen,
                decisions,
            };
            return probed(probe, input, right, run, scratch, output);
        }
        Lowering::Group { group, .. } => {
            let scratch = GroupScratch {
                index: &mut buffers.index,
                row: &mut buffers.row,
                values: &mut buffers.values,
                written: &mut buffers.written,
            };
            grouped(input, &group.keys, group.distinct, scratch, output)?;
        }
        Lowering::Lookup { .. } | Lowering::Unlowered => {}
    }
    Ok(None)
}
