//! Each streaming stage of a chain: its preparation before any row, in its held kernel's order, and its work on one input
//! row, each the kernel's own per-row function. Every stage writes through its own state, never another stage's.
use super::super::super::types::{ChainRows, ChainStage, StagePrepared};
use super::super::sources::service::{in_place, pending_of, rows_count, rows_kinds};
use crate::domain::comparing::types::NameCount;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::extend::item::{
    extended, kind_of, resolved,
};
use crate::domain::physical_plan::collections::kernels::filter::item::{
    filter, keep, kept_kinds, readied,
};
use crate::domain::physical_plan::collections::kernels::hash_index::item::emptied;
use crate::domain::physical_plan::collections::kernels::probe::item::{answer_kinds, probe_row};
use crate::domain::physical_plan::collections::kernels::probe_right::item::{
    ProbeScratch, probe_prepared,
};
use crate::domain::physical_plan::collections::kernels::project::item::projected;
use crate::domain::physical_plan::collections::kernels::text_map::item::{
    RowMapRun, mapped_kinds, row_mapped,
};
use crate::domain::physical_plan::service::indexes::service::{probe_build, probe_index};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::stream_scratch::value::{ProbeWork, StageOwn};
use crate::domain::physical_plan::types::{InputRows, KernelRun, NodeFed, ProbeIndex};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// The output kinds of stage `node`, lowered to `lowering`, over input rows of `input`, and what its preparation left, in
/// its held kernel's order: an extension's values resolved before its kinds, left empty when they fault; a probe's right
/// side's fault before its kinds, which it leaves empty. A filter's memo is readied for the file's `names`; a tail's
/// repeats are emptied. A probe does its right side's work only at its first left row, as its held kernel does none for a
/// left of no row.
///
/// # Errors
///
/// A stage's shape fault: a kept, projected or answered column past its input rows.
pub(in crate::domain::physical_plan::service::run::pipeline) fn stage_prepared(
    run: &mut KernelRun<'_>,
    buffers: &mut FileBuffers,
    stage: ChainStage<'_>,
    input: &[CellKind],
    names: NameCount,
    own: &mut StageOwn,
    right: Option<&ChainRows<'_>>,
) -> Result<StagePrepared, PortableOperatorFault> {
    let ChainStage { node, lowering } = stage;
    Ok(match lowering {
        Lowering::Filter { select, .. } => {
            let kinds = kept_kinds(input, &select.kept)?;
            if let Some(decisions) = buffers
                .decisions
                .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
            {
                readied(select, decisions, names);
            }
            StagePrepared { kinds, fault: None }
        }
        Lowering::TextMap { map, .. } => StagePrepared {
            kinds: mapped_kinds(map, input),
            fault: None,
        },
        // A projection keeps exactly its columns: none is a row of no column, never every column as a filter's none is.
        Lowering::Project { project, .. } => {
            let kinds = project
                .columns
                .iter()
                .map(|column| {
                    input
                        .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                        .copied()
                        .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })
                })
                .collect::<Result<_, _>>()?;
            emptied(&mut own.repeats);
            StagePrepared { kinds, fault: None }
        }
        // An extension's values are made before its rows are read: making a name changes the file's names, and reading
        // reads them.
        Lowering::Extend { extension, .. } => match resolved(
            &extension.values,
            &extension.bindings,
            run.resources,
            &mut own.values,
        ) {
            Ok(()) => StagePrepared {
                kinds: input
                    .iter()
                    .copied()
                    .chain(own.values.iter().map(|value| kind_of(*value)))
                    .collect(),
                fault: None,
            },
            Err(fault) => StagePrepared {
                kinds: Vec::new(),
                fault: Some(fault),
            },
        },
        // A probe reads every right row before it answers one: its right side's fault comes before any of its rows.
        Lowering::Probe { probe, index, .. } => {
            let right = right.ok_or(PortableOperatorFault::WrongOperation)?;
            // The probe's own state for this run, whatever it answers: what it is fed from its right side (its rows, as
            // read before their fault, or, walking the tree, the rows its walks find), no answer kept and no work done.
            own.fed = if *index == ProbeIndex::Tree {
                RowCount(0)
            } else {
                rows_count(right)
            };
            emptied(&mut own.repeats);
            own.work = ProbeWork::Pending;
            if let Some(fault) = pending_of(right) {
                return Ok(StagePrepared {
                    kinds: Vec::new(),
                    fault: Some(fault),
                });
            }
            let kinds = answer_kinds(probe, input, rows_kinds(right))?;
            StagePrepared { kinds, fault: None }
        }
        Lowering::Lookup { .. }
        | Lowering::Distinct { .. }
        | Lowering::Group { .. }
        | Lowering::Reach { .. }
        | Lowering::SeededReach { .. }
        | Lowering::Axis { .. }
        | Lowering::OrderedWalk { .. }
        | Lowering::Worklist { .. }
        | Lowering::Components { .. }
        | Lowering::OrderedMatch { .. }
        | Lowering::Render { .. }
        | Lowering::ScanBound { .. }
        | Lowering::Unlowered => return Err(PortableOperatorFault::WrongOperation),
    })
}

/// Runs one input row, `cells` of `kinds`, through stage `node`, lowered to `lowering`, into `out`, and answers the row's
/// fault: the rows it wrote before the fault stay. A probe does its work before its first left row at its first.
///
/// # Errors
///
/// A probe's hard fault: a right key column past its rows, an order that is no number.
#[expect(
    clippy::too_many_arguments,
    reason = "one row of one stage: the run, the buffers, the stage, its input row, its own state, its right rows, its rows"
)]
pub(in crate::domain::physical_plan::service::run::pipeline) fn stage_row(
    run: &mut KernelRun<'_>,
    buffers: &mut FileBuffers,
    stage: ChainStage<'_>,
    kinds: &[CellKind],
    cells: &[Value],
    own: &mut StageOwn,
    right: Option<&ChainRows<'_>>,
    out: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let ChainStage { node, lowering } = stage;
    let place = usize::try_from(node.0).unwrap_or(usize::MAX);
    Ok(match lowering {
        Lowering::Filter { select, .. } => {
            let decisions = buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            let kernel = filter(select, kinds, run.resources.texts());
            keep(
                kernel,
                run.matcher,
                cells,
                decisions,
                &mut buffers.written,
                out,
            )
            .err()
        }
        Lowering::TextMap { map, .. } => {
            let decisions = buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            row_mapped(
                map,
                cells,
                &mut RowMapRun {
                    resources: &mut *run.resources,
                    matcher: run.matcher,
                    decisions,
                    scratch: &mut buffers.scratch,
                    places: &mut buffers.places,
                    output: out,
                },
            )
            .err()
        }
        Lowering::Project { project, .. } => projected(
            cells,
            &project.columns,
            project.distinct,
            &mut own.repeats,
            out,
        )
        .err(),
        Lowering::Extend { .. } => {
            extended(cells, &own.values, &mut buffers.written, out);
            None
        }
        Lowering::Probe { probe, index, .. } => {
            let right = right.ok_or(PortableOperatorFault::WrongOperation)?;
            let run: &KernelRun<'_> = run;
            let mut tree = None;
            let right = in_place(right, run.resources.tree(), &mut tree);
            let StageOwn {
                index: own_index,
                kept,
                whole: [left_on, right_on],
                repeats,
                work,
                fed,
                ..
            } = own;
            let build = probe_build(&buffers.shared, *index);
            #[cfg(test)]
            if *work == ProbeWork::Pending
                && build == crate::domain::physical_plan::collections::kernels::probe_right::item::IndexBuild::Builds
            {
                buffers.builds.0 += 1;
            }
            let index = probe_index(&mut buffers.shared, own_index, *index)?;
            let mut scratch = ProbeScratch {
                index,
                build,
                kept,
                whole: [left_on, right_on],
                row: &mut buffers.row,
                written: &mut buffers.written,
                repeats,
                decisions: buffers
                    .decisions
                    .get_mut(place)
                    .ok_or(PortableOperatorFault::WrongOperation)?,
                found: &mut buffers.found,
            };
            if *work == ProbeWork::Pending {
                *work = ProbeWork::Done;
                if let Some(fault) = probe_prepared(probe, kinds, right, run, &mut scratch)? {
                    return Ok(Some(fault));
                }
            }
            let mut walked = NodeFed([RowCount(0), *fed, RowCount(0)]);
            let answered = probe_row(
                probe,
                InputRows::Row(cells, kinds),
                RowIndex(0),
                right,
                run,
                &mut scratch,
                &mut walked,
                out,
            );
            let [_, right_fed, _] = walked.0;
            *fed = right_fed;
            answered?
        }
        Lowering::Lookup { .. }
        | Lowering::Distinct { .. }
        | Lowering::Group { .. }
        | Lowering::Reach { .. }
        | Lowering::SeededReach { .. }
        | Lowering::Axis { .. }
        | Lowering::OrderedWalk { .. }
        | Lowering::Worklist { .. }
        | Lowering::Components { .. }
        | Lowering::OrderedMatch { .. }
        | Lowering::Render { .. }
        | Lowering::ScanBound { .. }
        | Lowering::Unlowered => return Err(PortableOperatorFault::WrongOperation),
    })
}
