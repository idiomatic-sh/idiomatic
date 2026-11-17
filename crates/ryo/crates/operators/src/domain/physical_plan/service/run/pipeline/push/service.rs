//! The chain's one row loop: each head row pushed through its stages in turn, each stage's chunk handed to its reader once it
//! reaches the bound, a stage that faults handing on the rows before its fault and reading no more.
use super::super::super::types::{ChainFault, ChainRows, ChainStage, Push};
use super::super::sources::service::{physical, resolved_rows, right_of, rows_count, rows_kinds};
use super::super::stages::service::stage_row;
use crate::domain::comparing::types::NameCount;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::filter::item::grown;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::decode;
use crate::domain::physical_plan::collections::kernels::scan_roots::item::{tree_read, tree_row};
use crate::domain::physical_plan::collections::kernels::text_map::item::reserved;
use crate::domain::physical_plan::service::run::counting::service::{counted, timed, timed_to};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::node_counters::value::MadeIn;
use crate::domain::physical_plan::types::stream_scratch::value::{
    STREAM_CHUNK_CELLS, StageScratch,
};
use crate::domain::physical_plan::types::{CellStart, NodeFed, PlanNodeId, TakenRows};
use ryo_language::domain::names::service::name_count;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// Records `fault` as the chain's hard fault when it comes before the one recorded: at an earlier node.
pub(in crate::domain::physical_plan::service::run::pipeline) fn first_fault(
    hard: &mut Option<ChainFault>,
    fault: ChainFault,
) {
    if hard.as_ref().is_none_or(|first| fault.node < first.node) {
        *hard = Some(fault);
    }
}

/// Pushes the head's rows through the prepared stages into the tail's table and answers the fault of the last stage that
/// stopped at one, the one the held plan's tail raises. Once a stage stops, the stages before it run on only when one of
/// them is a probe, which may still raise a hard fault the held plan raises first.
///
/// # Errors
///
/// None of its own: a stage's hard fault is the chain's, recorded at its place.
pub(in crate::domain::physical_plan::service::run::pipeline) fn pushed_all(
    push: &mut Push<'_, '_, '_>,
    nodes: &[PlanNodeId],
    row: &mut Vec<Value>,
    stages: &mut [StageScratch],
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let length = nodes.len();
    let head_rows = rows_count(push.head);
    // A stage alone on its head that writes one row per row it reads has its table sized once.
    if let [node] = nodes
        && let Lowering::TextMap { map, .. } = &physical(push.plan, *node)?.lowering
    {
        reserved(map, head_rows, push.tail);
    }
    // When counting is on, the head stage's time is the head loop's, less the time of the stages its rows are handed to.
    let span = timed(push.buffers);
    let looped = head_loop(push, nodes, row, stages);
    if let Some(head_node) = nodes.first() {
        timed_to(push.buffers, span, *head_node);
    }
    looped?;
    // The chunks left: each stage's rows to its reader, the head stage's first.
    for at in 0..length {
        let (Some(readers), Some(stages)) = (nodes.get(at + 1..), stages.get_mut(at..)) else {
            break;
        };
        if let Some((stage, after)) = stages.split_first_mut() {
            flushed(push, readers, stage, after)?;
        }
    }
    // What each stage's node was fed, as the held plan records it for the node it runs: the head stage its head's every row,
    // a later stage the rows it read, and a probe its right rows too. A fused node is counted as a held one is.
    for (at, (node, stage)) in nodes.iter().zip(stages.iter()).enumerate() {
        let left = if at == 0 { head_rows } else { stage.read };
        let right = if physical(push.plan, *node)
            .is_ok_and(|node| matches!(node.lowering, Lowering::Probe { .. }))
        {
            stage.own.fed
        } else {
            RowCount(0)
        };
        if let Some(fed) = push
            .buffers
            .fed
            .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
        {
            *fed = NodeFed([left, right, RowCount(0)]);
        }
        // A stage before the tail wrote the rows its reader read; the tail's rows are counted when they are held.
        if let Some(reader) = stages.get(at + 1) {
            counted(push.buffers, *node, MadeIn::Chain, reader.read);
        }
    }
    Ok(stages.iter_mut().rev().find_map(|stage| stage.fault.take()))
}

/// Pushes every head row through the stages, until a stage before the tail stops and no probe before it may still fault.
///
/// # Errors
///
/// A stage the plan does not hold.
fn head_loop(
    push: &mut Push<'_, '_, '_>,
    nodes: &[PlanNodeId],
    row: &mut Vec<Value>,
    stages: &mut [StageScratch],
) -> Result<(), PortableOperatorFault> {
    let length = nodes.len();
    let (head_kinds, head_rows) = (rows_kinds(push.head), rows_count(push.head));
    let first_probe = nodes.iter().position(|node| {
        physical(push.plan, *node).is_ok_and(|node| matches!(node.lowering, Lowering::Probe { .. }))
    });
    for at in 0..head_rows.0 {
        // A stopped tail is still fed every row its input makes, as the held plan feeds it: only a stage before the tail
        // that stopped ends the rows that reach it.
        if let Some(stopped) = stages
            .iter()
            .take(length.saturating_sub(1))
            .position(|stage| stage.fault.is_some())
            && first_probe.is_none_or(|probe| probe >= stopped)
        {
            break;
        }
        let head = push.head;
        let cells: &[Value] = match head {
            ChainRows::Facts(view) => {
                decode(view, RowIndex(at), row);
                row
            }
            ChainRows::Tree(roots) => {
                tree_row(
                    &tree_read(*roots, push.run.resources.tree()),
                    RowIndex(at),
                    row,
                );
                row
            }
            ChainRows::Copied(table) | ChainRows::Node(_, TakenRows { rows: table, .. }) => {
                row_at(table, RowIndex(at))
            }
            ChainRows::Held(table) => row_at(table, RowIndex(at)),
            ChainRows::Head | ChainRows::Right(_) | ChainRows::Untaken(_) => &[],
        };
        pushed(push, nodes, stages, head_kinds, cells)?;
    }
    Ok(())
}

/// The cells of `row` of `table`.
pub(in crate::domain::physical_plan::service::run::pipeline) fn row_at(
    table: &Table,
    row: RowIndex,
) -> &[Value] {
    let width = table.kinds.len().max(1);
    table
        .cells
        .get(row.0 * width..row.0 * width + width)
        .unwrap_or_default()
}

/// Pushes one input row, of `kinds`, through the first stage of `stages` (the stage of `nodes`' first node): its rows are
/// written to its chunk, or the tail's table for the last stage, and a chunk that reached the bound is handed on. A stage
/// that faults hands on the rows it wrote before the fault and reads no row after it; a hard fault is recorded at its place.
///
/// # Errors
///
/// A stage the plan does not hold.
pub(in crate::domain::physical_plan::service::run::pipeline) fn pushed(
    push: &mut Push<'_, '_, '_>,
    nodes: &[PlanNodeId],
    stages: &mut [StageScratch],
    kinds: &[CellKind],
    cells: &[Value],
) -> Result<(), PortableOperatorFault> {
    let (Some((node, after_nodes)), Some((stage, after))) =
        (nodes.split_first(), stages.split_first_mut())
    else {
        return Ok(());
    };
    // Every row handed to a stage is a row it is fed, before or after its fault; it reads none after its fault.
    stage.read = RowCount(stage.read.0.saturating_add(1));
    if stage.fault.is_some() {
        return Ok(());
    }
    let lowering = &physical(push.plan, *node)?.lowering;
    let right = right_of(lowering).and_then(|_| {
        push.rights
            .get(stage.own.right.0)
            .map(|rows| resolved_rows(rows, push.head, push.rights))
    });
    let out: &mut Table = if after.is_empty() {
        &mut *push.tail
    } else {
        &mut stage.out
    };
    stage.last = CellStart(out.cells.len());
    let fault = match stage_row(
        push.run,
        push.buffers,
        ChainStage {
            node: *node,
            lowering,
        },
        kinds,
        cells,
        &mut stage.own,
        right,
        out,
    ) {
        Ok(fault) => fault,
        Err(fault) => {
            first_fault(
                &mut push.hard,
                ChainFault {
                    node: *node,
                    fault: fault.clone(),
                },
            );
            Some(fault)
        }
    };
    if let Some(fault) = fault {
        stage.fault = Some(fault);
        return flushed(push, after_nodes, stage, after);
    }
    if !after.is_empty() && stage.out.cells.len() >= STREAM_CHUNK_CELLS {
        return flushed(push, after_nodes, stage, after);
    }
    Ok(())
}

/// Hands `stage`'s chunk to its reader, the first of `after` (the stages of `nodes`), row by row, and empties it. The reader's
/// memo first grows to the names the stages before it minted.
///
/// # Errors
///
/// A stage the plan does not hold.
pub(in crate::domain::physical_plan::service::run::pipeline) fn flushed(
    push: &mut Push<'_, '_, '_>,
    nodes: &[PlanNodeId],
    stage: &mut StageScratch,
    after: &mut [StageScratch],
) -> Result<(), PortableOperatorFault> {
    let Some(reader) = nodes.first() else {
        return Ok(());
    };
    #[cfg(test)]
    if stage.last.0 >= STREAM_CHUNK_CELLS {
        push.buffers.chunks =
            crate::domain::physical_plan::types::file_buffers::value::ChunkMeter::Over;
    }
    if let Lowering::Filter { select, .. } = &physical(push.plan, *reader)?.lowering
        && let Some(decisions) = push
            .buffers
            .decisions
            .get_mut(usize::try_from(reader.0).unwrap_or(usize::MAX))
    {
        let names = NameCount(name_count(push.run.resources.texts().symbols).0);
        grown(select, decisions, names);
    }
    // When counting is on, the reader's time is the chunk's, less the time of the stages after it.
    let span = timed(push.buffers);
    let handed = (0..stage.out.count.0).try_for_each(|at| {
        pushed(
            push,
            nodes,
            after,
            &stage.out.kinds,
            row_at(&stage.out, RowIndex(at)),
        )
    });
    timed_to(push.buffers, span, *reader);
    handed?;
    stage.out.cells.clear();
    stage.out.count = RowCount(0);
    stage.last = CellStart(0);
    Ok(())
}
