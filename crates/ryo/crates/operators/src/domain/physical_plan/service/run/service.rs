//! The runner: a node's kernels over one file's rows. A node keeps the rows it made before its first fault and raises the fault
//! after them; its reader reads every row before an input's fault and raises that fault only then, unless one of those rows
//! faults first, so the earliest row's fault wins whatever the pipeline (D15).
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::distinct::item::counted;
use crate::domain::physical_plan::collections::kernels::extend::item::{
    extended, kind_of, resolved,
};
use crate::domain::physical_plan::collections::kernels::filter::item::{filter, keep, kept_kinds};
use crate::domain::physical_plan::collections::kernels::group::item::{GroupScratch, grouped};
use crate::domain::physical_plan::collections::kernels::hash_index::item::emptied;
use crate::domain::physical_plan::collections::kernels::lookup::item::lookups;
use crate::domain::physical_plan::collections::kernels::ordered_match::item::matched;
use crate::domain::physical_plan::collections::kernels::probe::item::probed;
use crate::domain::physical_plan::collections::kernels::project::item::projected;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::{
    count_of, kinds_of, row_of,
};
use crate::domain::physical_plan::collections::kernels::scan_node::item::{hold, sources_of};
use crate::domain::physical_plan::service::indexes::service::{
    closed, opened, probe_scratch, sized_shared,
};
use crate::domain::physical_plan::service::run::reading::service::{
    pending_of, reading, returned, rows_in,
};
use crate::domain::physical_plan::service::run::types::KernelInputs;
use crate::domain::physical_plan::types::file_buffers::value::{FileBuffers, NodeDecisions};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    CellStart, KernelRun, Lowering, NodeReaders, NodeRows, PlanNodeId, ProbeIndex, RowSource,
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
    sized_shared(plan, buffers);
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

/// Runs `node`'s kernels and holds its rows and pending fault; a lookup's build answers every lookup of its index. A probe of
/// a shared index counts down its probes in the file whatever it answers, and the last empties the index.
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
    let Lowering::Probe {
        index: ProbeIndex::Shared(at),
        ..
    } = lowering
    else {
        return made(plan, run, node, lowering, buffers);
    };
    opened(plan, buffers, *at);
    let answered = made(plan, run, node, lowering, buffers);
    closed(buffers, *at);
    answered
}

/// Runs the kernels of `node`, lowered to `lowering`, over its inputs, and holds its rows and pending fault.
fn made(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    node: PlanNodeId,
    lowering: &Lowering,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    match lowering {
        Lowering::Lookup { index, .. } => return lookups(plan, run, *index, buffers),
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
    }
    let mut sources = sources_of(lowering);
    let first = sources
        .next()
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let second = sources.next();
    ready(plan, run, first, buffers)?;
    if let Some(second) = second {
        ready(plan, run, second, buffers)?;
    }
    if let Lowering::TextMap { .. } | Lowering::Render { .. } = lowering {
        return super::minted::service::minted(plan, run, node, lowering, first, buffers);
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
        | Lowering::TextMap { .. }
        | Lowering::OrderedMatch { .. }
        | Lowering::Render { .. }
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
        Lowering::Group { .. } | Lowering::Render { .. } => left_fault.clone(),
        Lowering::OrderedMatch { .. } => left_fault.clone().or_else(|| right_fault.clone()),
        Lowering::Filter { .. }
        | Lowering::Project { .. }
        | Lowering::Extend { .. }
        | Lowering::Distinct { .. }
        | Lowering::TextMap { .. }
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
            kernels(
                lowering,
                run,
                node,
                inputs,
                resolution,
                buffers,
                &mut output,
            )
        }
    };
    returned(plan, buffers, left, NodeReaders(if shared { 2 } else { 1 }));
    if let Some(right) = right {
        returned(plan, buffers, right, NodeReaders(1));
    }
    let fault = made?;
    hold(
        plan,
        buffers,
        node,
        output,
        fault.or(left_fault).or(right_fault),
    );
    Ok(())
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
                if let Err(fault) = keep(
                    kernel,
                    run.matcher,
                    cells,
                    decisions,
                    &mut buffers.written,
                    output,
                ) {
                    return Ok(Some(fault));
                }
            }
        }
        Lowering::Project { project, .. } => {
            // A projection keeps exactly its columns: none is a row of no column, never every column as a filter's none is.
            output.kinds = project
                .columns
                .iter()
                .map(|column| {
                    kinds
                        .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                        .copied()
                        .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })
                })
                .collect::<Result<_, _>>()?;
            emptied(&mut buffers.repeats);
            for row in 0..rows {
                let cells = row_of(input, RowIndex(row), &mut buffers.row);
                if let Err(fault) = projected(
                    cells,
                    &project.columns,
                    project.distinct,
                    &mut buffers.repeats,
                    output,
                ) {
                    return Ok(Some(fault));
                }
            }
        }
        Lowering::Extend { .. } => {
            if let Some(fault) = resolution {
                return Ok(Some(fault));
            }
            output.kinds = kinds
                .iter()
                .copied()
                .chain(buffers.values.iter().map(|value| kind_of(*value)))
                .collect();
            for row in 0..rows {
                let cells = row_of(input, RowIndex(row), &mut buffers.row);
                extended(cells, &buffers.values, &mut buffers.written, output);
            }
        }
        Lowering::Distinct { .. } => {
            output.kinds = kinds.to_vec();
            emptied(&mut buffers.repeats);
            for source in std::iter::once(input).chain(other) {
                if kinds_of(source) != kinds {
                    return Err(PortableOperatorFault::ShapeMismatch);
                }
                for row in 0..count_of(source).0 {
                    let start = CellStart(output.cells.len());
                    output
                        .cells
                        .extend_from_slice(row_of(source, RowIndex(row), &mut buffers.row));
                    counted(
                        output,
                        start,
                        ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
                        &mut buffers.repeats,
                    );
                }
            }
        }
        Lowering::Probe { probe, index, .. } => {
            let right = other.ok_or(PortableOperatorFault::WrongOperation)?;
            let scratch = probe_scratch(buffers, *index, node)?;
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
        Lowering::OrderedMatch { ordered, .. } => {
            let right = other.ok_or(PortableOperatorFault::WrongOperation)?;
            if let Err(fault) = matched(ordered, input, right, run.resources.texts(), output) {
                return Ok(Some(fault));
            }
        }
        // Run by `minted`, which holds the resources mutably while it reads its rows.
        Lowering::TextMap { .. }
        | Lowering::Render { .. }
        | Lowering::Lookup { .. }
        | Lowering::Unlowered => {}
    }
    Ok(None)
}
