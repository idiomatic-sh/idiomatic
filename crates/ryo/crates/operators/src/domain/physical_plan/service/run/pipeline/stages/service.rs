//! Each streaming stage of a chain: its preparation before any row, in its held kernel's order, and its work on a chunk of
//! input rows, resolved once for the chunk, each row the kernel's own per-row function. Every stage writes through its own
//! state, never another stage's.
use super::super::super::types::{ChainRows, ChainStage, StagePrepared};
use super::super::sources::service::{in_place, pending_of, rows_count, rows_kinds};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::extend::item::{
    extended, kind_of, resolved,
};
use crate::domain::physical_plan::collections::kernels::filter::item::{
    filter, keep, kept_kinds, readied,
};
use crate::domain::physical_plan::collections::kernels::probe::item::{answer_kinds, probe_rows};
use crate::domain::physical_plan::collections::kernels::probe_right::item::{
    ProbeScratch, probe_prepared,
};
use crate::domain::physical_plan::collections::kernels::project::item::projected;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::{
    chunk_read, count_of, kinds_of, row_of,
};
use crate::domain::physical_plan::collections::kernels::text_map::item::{
    RowMapRun, mapped_kinds, row_mapped,
};
use crate::domain::physical_plan::service::indexes::service::{probe_build, probe_index};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::stream_scratch::value::{
    CellBound, ChunkRead, ChunkStop, ProbeWork, RowChunk, StageOwn,
};
use crate::domain::physical_plan::types::{KernelRun, NodeFed, ProbeIndex};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::rows::types::{RowCount, Table};

/// The output kinds of stage `node`, lowered to `lowering`, over input rows of `input`, and what its preparation left, in
/// its held kernel's order: an extension's values resolved before its kinds, left empty when they fault; a probe's right
/// side's fault before its kinds, which it leaves empty. A filter's memo is readied; a stage's repeats, and a probe's own
/// index, are readied with room from the worker's index pool. A probe does its right side's work only at its first left row, as its held kernel does none for a
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
                readied(decisions);
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
            own.repeats.cleared();
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
            own.repeats.cleared();
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

/// Runs the rows of `input` from its first unread row through stage `stage` into `out`, until every row is read, a row
/// faults, or `out` holds `bound` cells or more after a row; the stage's per-run work (its kernel, its decisions, its right
/// rows) is resolved once for the call, never per row, and each row is the kernel's own per-row function's. The rows a row
/// wrote before its fault stay. A probe does its work before its first left row at the first call that reads one.
///
/// # Errors
///
/// A stage the chain does not stream; a probe's hard fault: a right key column past its rows, an order that is no number.
#[expect(
    clippy::too_many_arguments,
    reason = "one chunk of one stage: the run, the buffers, the stage, its rows, its own state, its right rows, its rows, its bound"
)]
pub(in crate::domain::physical_plan::service::run::pipeline) fn stage_chunk(
    run: &mut KernelRun<'_>,
    buffers: &mut FileBuffers,
    stage: ChainStage<'_>,
    input: RowChunk<'_>,
    own: &mut StageOwn,
    right: Option<&ChainRows<'_>>,
    out: &mut Table,
    bound: CellBound,
) -> Result<ChunkRead, PortableOperatorFault> {
    let ChainStage { node, lowering } = stage;
    let place = usize::try_from(node.0).unwrap_or(usize::MAX);
    let kinds = kinds_of(input.rows);
    Ok(match lowering {
        Lowering::Filter { select, .. } => {
            let decisions = buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            let kernel = filter(select, kinds, run.resources.texts());
            let written = &mut buffers.written;
            each_row(input, &mut buffers.row, out, bound, |cells, out| {
                keep(kernel, run.matcher, cells, decisions, written, out)
            })?
        }
        Lowering::TextMap { map, .. } => {
            let decisions = buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            let (scratch, places) = (&mut buffers.scratch, &mut buffers.places);
            let matcher = run.matcher;
            let resources = &mut *run.resources;
            each_row(input, &mut buffers.row, out, bound, |cells, out| {
                row_mapped(
                    map,
                    cells,
                    &mut RowMapRun {
                        resources: &mut *resources,
                        matcher,
                        decisions: &mut *decisions,
                        scratch: &mut *scratch,
                        places: &mut *places,
                        output: out,
                    },
                )
            })?
        }
        Lowering::Project { project, .. } => {
            let repeats = &mut own.repeats;
            each_row(input, &mut buffers.row, out, bound, |cells, out| {
                projected(cells, &project.columns, project.distinct, repeats, out)
            })?
        }
        Lowering::Extend { .. } => {
            let (values, written) = (&own.values, &mut buffers.written);
            each_row(input, &mut buffers.row, out, bound, |cells, out| {
                extended(cells, values, written, out);
                Ok(())
            })?
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
            let build = probe_build(&buffers.shared.indexes, *index);
            #[cfg(test)]
            if *work == ProbeWork::Pending
                && build == crate::domain::physical_plan::collections::kernels::probe_right::item::IndexBuild::Builds
            {
                buffers.builds.0 += 1;
            }
            let index = probe_index(&mut buffers.shared.indexes, own_index, *index)?;
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
            // The probe's work before its first left row: its right filter, its index build, its extremes. Its fault is
            // its first left row's.
            if *work == ProbeWork::Pending && count_of(input.rows).0 > input.from.0 {
                *work = ProbeWork::Done;
                if let Some(fault) = probe_prepared(probe, kinds, right, run, &mut scratch)? {
                    return Ok(ChunkRead {
                        read: RowCount(1),
                        stop: ChunkStop::Faulted(fault),
                    });
                }
            }
            let mut walked = NodeFed([RowCount(0), *fed, RowCount(0)]);
            let answered = probe_rows(
                probe,
                input,
                right,
                run,
                &mut scratch,
                &mut walked,
                out,
                bound,
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

/// Runs `row`, a kernel's per-row function, over the rows of `input` from its first unread row into `out`, as `chunk_read`
/// reads them: a row's error is its fault. A row of a relation or the tree is decoded into `scratch`; a
/// table's is read in place.
fn each_row(
    input: RowChunk<'_>,
    scratch: &mut Vec<Value>,
    out: &mut Table,
    bound: CellBound,
    mut row: impl FnMut(&[Value], &mut Table) -> Result<(), PortableOperatorFault>,
) -> Result<ChunkRead, PortableOperatorFault> {
    chunk_read(input, out, bound, |at, out| {
        Ok(row(row_of(input.rows, at, scratch), out).err())
    })
}
