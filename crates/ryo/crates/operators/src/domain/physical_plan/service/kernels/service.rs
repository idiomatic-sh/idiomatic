//! The runner's kernels: the one dispatch from a node's lowering to the kernels that make its rows from its inputs'
//! rows.
use super::super::run::types::KernelInputs;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::components::item::cyclic;
use crate::domain::physical_plan::collections::kernels::directed_walk::item::{
    AxisInputs, AxisTables, walked,
};
use crate::domain::physical_plan::collections::kernels::distinct::item::counted;
use crate::domain::physical_plan::collections::kernels::group::item::{GroupScratch, grouped};
use crate::domain::physical_plan::collections::kernels::ordered_match::item::matched;
use crate::domain::physical_plan::collections::kernels::ordered_walk::item::visited;
use crate::domain::physical_plan::collections::kernels::probe_right::item::{
    IndexBuild, ProbeScratch,
};
use crate::domain::physical_plan::collections::kernels::queued_walk::item::{
    WorklistInputs, worklisted,
};
use crate::domain::physical_plan::collections::kernels::reach::item::{
    ReachScratch, Starts, reached,
};
use crate::domain::physical_plan::collections::kernels::scan_facts::item::{
    count_of, kinds_of, row_of,
};
use crate::domain::physical_plan::service::indexes::service::closed;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::shared_index::value::{
    IndexKind, IndexRows, KeyedAccess, KeyedStorage,
};
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
    buffers: &mut FileBuffers,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let place = usize::try_from(node.0).unwrap_or(usize::MAX);
    let kinds = kinds_of(input);
    match lowering {
        Lowering::Distinct { .. } => {
            output.kinds = kinds.to_vec();
            buffers.pools.rows.refilled(&mut buffers.repeats);
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
        Lowering::Group { group, .. } => {
            buffers.pools.keyed.refilled(&mut buffers.index);
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
            let (edges, starts) = if let Lowering::SeededReach { join, .. } = lowering {
                (
                    other.ok_or(PortableOperatorFault::WrongOperation)?,
                    Starts::Seeded(input, join),
                )
            } else {
                (input, Starts::Every)
            };
            let decisions = buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            // Spike (ryo-fmx3): a seeded walk over facts reads the file's shared index of its edges.
            let shared = match lowering {
                Lowering::SeededReach { edge_index, .. } => *edge_index,
                _ => None,
            };
            if shared.is_none() {
                buffers.pools.keyed.refilled(&mut buffers.index);
            }
            buffers.pools.rows.refilled(&mut buffers.repeats);
            // The seeded join's own index, right rows and key columns, for this run.
            if let Lowering::SeededReach { join, .. } = lowering {
                buffers
                    .second
                    .readied(IndexKind::of_probe(&join.probe.key), &mut buffers.pools);
            }
            let (mut kept, mut left_on, mut right_on) = (Table::default(), Vec::new(), Vec::new());
            let storage = match shared {
                Some(reader) => KeyedStorage::File {
                    indexes: &mut buffers.shared,
                    pools: &mut buffers.pools,
                    reader,
                },
                None => KeyedStorage::Own(&mut buffers.index),
            };
            let scratch = ReachScratch {
                index: KeyedAccess {
                    storage,
                    #[cfg(test)]
                    builds: &mut buffers.builds,
                },
                walk: &mut buffers.walk,
                probe: ProbeScratch {
                    index: &mut buffers.second,
                    build: IndexBuild::Builds,
                    kept: &mut kept,
                    whole: [&mut left_on, &mut right_on],
                    row: &mut buffers.row,
                    written: &mut buffers.written,
                    repeats: &mut buffers.repeats,
                    decisions,
                    found: &mut buffers.found,
                },
            };
            let answer = reached(graph, edges, starts, run, scratch, output);
            if let Some(reader) = shared {
                closed(buffers, reader.place);
            }
            return answer;
        }
        Lowering::Axis {
            axis, edge_index, ..
        } => {
            let inputs = AxisInputs {
                starts: input,
                edges: other.ok_or(PortableOperatorFault::WrongOperation)?,
                domain: third.ok_or(PortableOperatorFault::WrongOperation)?,
            };
            let decisions = buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            // The domain is keyed: the second index is readied as keyed rows.
            buffers.second.readied(IndexKind::Keyed, &mut buffers.pools);
            let IndexRows::Keyed(domain) = &mut buffers.second else {
                return Err(PortableOperatorFault::WrongOperation);
            };
            let storage = match edge_index {
                Some(reader) => KeyedStorage::File {
                    indexes: &mut buffers.shared,
                    pools: &mut buffers.pools,
                    reader: *reader,
                },
                None => {
                    buffers.pools.keyed.refilled(&mut buffers.index);
                    KeyedStorage::Own(&mut buffers.index)
                }
            };
            let tables = AxisTables {
                edges: KeyedAccess {
                    storage,
                    #[cfg(test)]
                    builds: &mut buffers.builds,
                },
                domain,
                decisions,
            };
            let answer = walked(axis, inputs, run, tables, &mut buffers.walk, output);
            if let Some(reader) = edge_index {
                closed(buffers, reader.place);
            }
            return answer;
        }
        Lowering::OrderedWalk { dfs, .. } => {
            let edges = other.ok_or(PortableOperatorFault::WrongOperation)?;
            return visited(
                dfs,
                input,
                edges,
                &run.resources.texts(),
                &mut buffers.walk,
                output,
            );
        }
        Lowering::Worklist { worklist, .. } => {
            buffers.pools.rows.refilled(&mut buffers.repeats);
            let inputs = WorklistInputs {
                roots: input,
                edges: other.ok_or(PortableOperatorFault::WrongOperation)?,
                lattice: third.ok_or(PortableOperatorFault::WrongOperation)?,
            };
            return worklisted(
                worklist,
                inputs,
                &run.resources.texts(),
                &mut buffers.worklist,
                &mut buffers.repeats,
                output,
            );
        }
        Lowering::Components { cycles, .. } => {
            buffers.pools.keyed.refilled(&mut buffers.index);
            return cyclic(
                cycles,
                input,
                &mut buffers.index,
                &mut buffers.cycles,
                output,
            );
        }
        Lowering::PointerChase { chase, .. } => {
            return crate::domain::physical_plan::collections::kernels::pointer_chase::item::chased(chase, input, output);
        }
        Lowering::OrderedMatch {
            ordered,
            left: left_source,
            right: right_source,
        } => {
            let right = other.ok_or(PortableOperatorFault::WrongOperation)?;
            buffers.pools.rows.refilled(&mut buffers.repeats);
            if let Err(fault) = matched(
                ordered,
                input,
                right,
                run.resources.texts(),
                left_source == right_source,
                &mut buffers.repeats,
                output,
            ) {
                return Ok(Some(fault));
            }
        }
        // A filter, a projection, an extension, a probe and a text_map stage are run by the chain of streamed stages, a render by
        // `minted`, which holds the resources mutably while it reads its rows.
        Lowering::Filter { .. }
        | Lowering::Project { .. }
        | Lowering::Extend { .. }
        | Lowering::Probe { .. }
        | Lowering::TextMap { .. }
        | Lowering::Render { .. }
        | Lowering::Lookup { .. }
        | Lowering::ScanBound { .. }
        | Lowering::Unlowered => {}
    }
    Ok(None)
}
