//! The runner's kernels: the one dispatch from a node's lowering to the kernels that make its rows from its inputs'
//! rows.
use super::super::run::types::KernelInputs;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::components::item::cyclic;
use crate::domain::physical_plan::collections::kernels::directed_walk::item::{
    AxisInputs, AxisTables, walked,
};
use crate::domain::physical_plan::collections::kernels::distinct::item::counted;
use crate::domain::physical_plan::collections::kernels::extend::item::{extended, kind_of};
use crate::domain::physical_plan::collections::kernels::filter::item::{filter, keep, kept_kinds};
use crate::domain::physical_plan::collections::kernels::group::item::{GroupScratch, grouped};
use crate::domain::physical_plan::collections::kernels::ordered_walk::item::visited;
use crate::domain::physical_plan::collections::kernels::probe::item::{ProbeScratch, probed};
use crate::domain::physical_plan::collections::kernels::project::item::projected;
use crate::domain::physical_plan::collections::kernels::reach::item::{
    ReachScratch, Starts, reached,
};
use crate::domain::physical_plan::collections::kernels::scan_facts::item::{
    count_of, kinds_of, row_of,
};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::{CellStart, KernelRun, PlanNodeId};
use ryo_values::domain::rows::types::{RowIndex, Table};

/// Makes a node's rows from its inputs' rows into `output`, and answers the fault of the first row that raised one.
///
/// # Errors
///
/// A fault of the node's shape: a column past its input's rows.
pub(crate) fn kernels(
    lowering: &Lowering,
    run: &KernelRun<'_>,
    node: PlanNodeId,
    KernelInputs {
        input,
        other,
        third,
    }: KernelInputs<'_>,
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
            buffers.seen.clear();
            for row in 0..rows {
                let cells = row_of(input, RowIndex(row), &mut buffers.row);
                if let Err(fault) = projected(
                    cells,
                    &project.columns,
                    project.distinct,
                    &mut buffers.seen,
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
            buffers.seen.clear();
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
                        &mut buffers.seen,
                    );
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
                hashes: &mut buffers.hashes,
                texts: run.resources.texts(),
                written: &mut buffers.written,
            };
            grouped(input, &group.keys, group.measure, scratch, output)?;
        }
        Lowering::Reach { graph, .. } | Lowering::SeededReach { graph, .. } => {
            let (edges, starts) = match lowering {
                Lowering::SeededReach { join, .. } => (
                    other.ok_or(PortableOperatorFault::WrongOperation)?,
                    Starts::Seeded(input, join),
                ),
                _ => (input, Starts::Every),
            };
            let decisions = buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            let scratch = ReachScratch {
                index: &mut buffers.index,
                walk: &mut buffers.walk,
                probe: ProbeScratch {
                    index: &mut buffers.second,
                    row: &mut buffers.row,
                    written: &mut buffers.written,
                    seen: &mut buffers.seen,
                    decisions,
                },
            };
            return reached(graph, edges, starts, run, scratch, output);
        }
        Lowering::Axis { axis, .. } => {
            let inputs = AxisInputs {
                starts: input,
                edges: other.ok_or(PortableOperatorFault::WrongOperation)?,
                domain: third.ok_or(PortableOperatorFault::WrongOperation)?,
            };
            let decisions = buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            let tables = AxisTables {
                edges: &mut buffers.index,
                domain: &mut buffers.second,
                decisions,
            };
            return walked(axis, inputs, run, tables, &mut buffers.walk, output);
        }
        Lowering::OrderedWalk { dfs, .. } => {
            let edges = other.ok_or(PortableOperatorFault::WrongOperation)?;
            return visited(dfs, input, edges, &mut buffers.walk, output);
        }
        Lowering::Components { cycles, .. } => {
            return cyclic(
                cycles,
                input,
                &mut buffers.index,
                &mut buffers.cycles,
                output,
            );
        }
        Lowering::Lookup { .. } | Lowering::Unlowered => {}
    }
    Ok(None)
}
