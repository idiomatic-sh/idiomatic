//! The chain's one chunk loop: each chunk of head rows handed to the head stage whole, each stage's chunk handed to its reader
//! once it reaches the bound, a stage that faults handing on the rows before its fault and reading no more.
use super::super::super::types::{ChainFault, ChainRows, ChainStage, HeadReads, Push};
use super::super::sources::service::{physical, resolved_rows, right_of, rows_count, rows_kinds};
use super::super::stages::service::stage_chunk;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::count_of;
use crate::domain::physical_plan::collections::kernels::scan_node::item::fed_with;
use crate::domain::physical_plan::collections::kernels::scan_roots::item::{tree_read, tree_row};
use crate::domain::physical_plan::collections::kernels::text_map::item::reserved;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::stream_scratch::value::{
    CellBound, ChunkRead, ChunkStop, RowChunk, STREAM_CHUNK_CELLS, StageScratch,
};
use crate::domain::physical_plan::types::{InputRows, NodeFed, PlanNodeId, TakenRows};
use ryo_relations::domain::relations::types::Value;
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

/// Whether the head stage reads on: not once a stage before the tail stopped with no probe at or before it. A stopped tail
/// is still fed every row its input makes, as the held plan feeds it; a probe before a stopped stage may still raise a hard
/// fault the held plan raises first.
fn head_reads(push: &Push<'_, '_, '_>, stages: &[StageScratch]) -> HeadReads {
    match stages
        .iter()
        .take(push.last.0)
        .position(|stage| stage.fault.is_some())
    {
        Some(stopped) if push.first_probe.is_none_or(|probe| probe.0 >= stopped) => {
            HeadReads::Ended
        }
        Some(_) | None => HeadReads::On,
    }
}

/// Pushes the head's rows through the prepared stages into the tail's table and answers the fault of the last stage that
/// stopped at one, the one the held plan's tail raises. A relation's columns or a held table are handed to the head stage
/// whole, in place; the tree's rows are decoded a chunk at a time into `head` (each row through `row`), the tree lent only
/// while a chunk is decoded, so a stage that mints between chunks holds no borrow of it. Once a stage stops, the head stage reads on only
/// when a probe comes at or before it.
///
/// # Errors
///
/// None of its own: a stage's hard fault is the chain's, recorded at its place.
pub(in crate::domain::physical_plan::service::run::pipeline) fn pushed_all(
    push: &mut Push<'_, '_, '_>,
    nodes: &[PlanNodeId],
    row: &mut Vec<Value>,
    head: &mut Table,
    stages: &mut [StageScratch],
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let length = nodes.len();
    let (head_kinds, head_rows) = (rows_kinds(push.head), rows_count(push.head));
    // A stage alone on its head that writes one row per row it reads has its table sized once.
    if let [node] = nodes
        && let Lowering::TextMap { map, .. } = &physical(push.plan, *node)?.lowering
    {
        reserved(map, head_rows, push.tail);
    }
    // A relation's columns and a held table are read in place, each row as the stage reads it (a relation's row decoded on
    // its own, never read past its own cells); the tree's rows are decoded a chunk at a time.
    let in_place = match push.head {
        ChainRows::Facts(view) => Some(InputRows::Facts(view)),
        ChainRows::Node(_, TakenRows { rows: table, .. }) => Some(InputRows::Table(table)),
        ChainRows::Held(table) => Some(InputRows::Table(table)),
        ChainRows::Tree(_) | ChainRows::Head | ChainRows::Right(_) | ChainRows::Untaken(_) => None,
    };
    match push.head {
        ChainRows::Facts(_) | ChainRows::Node(..) | ChainRows::Held(_) => {
            if let Some(rows) = in_place
                && head_rows.0 > 0
                && head_reads(push, stages) == HeadReads::On
            {
                pushed_chunk(push, nodes, stages, rows)?;
            }
        }
        ChainRows::Tree(roots) => {
            head.kinds.clear();
            head.kinds.extend_from_slice(head_kinds);
            // Whole rows, as many as fit the bound, and at least one; a tree row holds a cell for every one of its kinds.
            let per_chunk = (STREAM_CHUNK_CELLS / head_kinds.len().max(1)).max(1);
            let mut at = 0;
            while at < head_rows.0 && head_reads(push, stages) == HeadReads::On {
                let end = head_rows.0.min(at.saturating_add(per_chunk));
                head.cells.clear();
                let tree = tree_read(*roots, push.run.resources.tree());
                for each in at..end {
                    tree_row(&tree, RowIndex(each), row);
                    head.cells.extend_from_slice(row);
                }
                head.count = RowCount(end - at);
                at = end;
                pushed_chunk(push, nodes, stages, InputRows::Table(head))?;
            }
        }
        ChainRows::Head | ChainRows::Right(_) | ChainRows::Untaken(_) => {}
    }
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
        fed_with(push.buffers, *node, NodeFed([left, right, RowCount(0)]));
    }
    Ok(stages.iter_mut().rev().find_map(|stage| stage.fault.take()))
}

/// Pushes the rows of `input` through the first stage of `stages` (the stage of `nodes`' first node), one call per run of
/// rows: the stage's rows go to its chunk, or the tail's table for the last stage; a chunk that reached the bound is handed
/// on whole before the stage reads on. A stage that faults hands on the rows it wrote before the fault and reads no row
/// after it; a hard fault is recorded at its place. Every row of `input` is a row the stage is fed, before or after its
/// fault. The head stage reads on after each chunk it handed on only as `head_reads` answers.
///
/// # Errors
///
/// A stage the plan does not hold.
pub(in crate::domain::physical_plan::service::run::pipeline) fn pushed_chunk(
    push: &mut Push<'_, '_, '_>,
    nodes: &[PlanNodeId],
    stages: &mut [StageScratch],
    input: InputRows<'_>,
) -> Result<(), PortableOperatorFault> {
    let head_stage = stages.len() == push.last.0.saturating_add(1);
    let (Some((node, after_nodes)), Some(stage)) = (nodes.split_first(), stages.first_mut()) else {
        return Ok(());
    };
    let count = count_of(input);
    stage.read = RowCount(stage.read.0.saturating_add(count.0));
    if stage.fault.is_some() {
        return Ok(());
    }
    let lowering = &physical(push.plan, *node)?.lowering;
    let (head, rights) = (push.head, push.rights);
    let tail = after_nodes.is_empty();
    let bound = CellBound(if tail { usize::MAX } else { STREAM_CHUNK_CELLS });
    let mut from = RowIndex(0);
    loop {
        let Some((stage, after)) = stages.split_first_mut() else {
            return Ok(());
        };
        let right = right_of(lowering).and_then(|_| {
            rights
                .get(stage.own.right.0)
                .map(|rows| resolved_rows(rows, head, rights))
        });
        let out: &mut Table = if tail {
            &mut *push.tail
        } else {
            &mut stage.out
        };
        #[cfg(test)]
        {
            push.buffers.chains.calls.0 += 1;
            if out.cells.len() >= bound.0 {
                push.buffers.chunks =
                    crate::domain::physical_plan::types::file_buffers::value::ChunkMeter::Over;
            }
        }
        let read = stage_chunk(
            push.run,
            push.buffers,
            ChainStage {
                node: *node,
                lowering,
            },
            RowChunk { rows: input, from },
            &mut stage.own,
            right,
            out,
            bound,
        )
        .unwrap_or_else(|fault| ChunkRead {
            read: RowCount(0),
            stop: ChunkStop::Hard(fault),
        });
        match read.stop {
            ChunkStop::Ended => {
                // A stage that read every row below its bound left its chunk below it.
                #[cfg(test)]
                if !tail && stage.out.cells.len() >= bound.0 {
                    push.buffers.chunks =
                        crate::domain::physical_plan::types::file_buffers::value::ChunkMeter::Over;
                }
                return Ok(());
            }
            ChunkStop::Full => {
                flushed(push, after_nodes, stage, after)?;
                from = RowIndex(from.0.saturating_add(read.read.0));
                // The head stage reads on as it would before its next head row: not once a stage before the tail stopped
                // with no probe at or before it.
                if from.0 >= count.0 || (head_stage && head_reads(push, stages) == HeadReads::Ended)
                {
                    return Ok(());
                }
            }
            ChunkStop::Faulted(fault) => {
                stage.fault = Some(fault);
                return flushed(push, after_nodes, stage, after);
            }
            ChunkStop::Hard(fault) => {
                first_fault(
                    &mut push.hard,
                    ChainFault {
                        node: *node,
                        fault: fault.clone(),
                    },
                );
                stage.fault = Some(fault);
                return flushed(push, after_nodes, stage, after);
            }
        }
    }
}

/// Hands `stage`'s chunk to its reader, the first of `after` (the stages of `nodes`), whole, and empties it.
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
    if nodes.is_empty() {
        return Ok(());
    }
    #[cfg(test)]
    {
        push.buffers.chains.handed = push.buffers.chains.handed.max(stage.out.count);
    }
    if stage.out.count.0 > 0 {
        pushed_chunk(push, nodes, after, InputRows::Table(&stage.out))?;
    }
    stage.out.cells.clear();
    stage.out.count = RowCount(0);
    Ok(())
}
