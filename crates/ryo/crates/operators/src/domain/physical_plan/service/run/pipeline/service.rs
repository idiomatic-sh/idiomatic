//! The runner's chain of streamed stages: the rows of the stages that feed a node, pushed through each stage in turn in
//! bounded chunks, only the node's own rows held. Each stage keeps the rows it wrote before its first fault and stops there;
//! its reader reads every row before that fault, so the earliest row's fault wins whatever the chunking (D15).
use super::super::types::StagePrepared;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::extend::item::{
    extended, kind_of, resolved,
};
use crate::domain::physical_plan::collections::kernels::filter::item::{
    filter, grown, keep, kept_kinds, readied,
};
use crate::domain::physical_plan::collections::kernels::hash_index::item::emptied;
use crate::domain::physical_plan::collections::kernels::project::item::projected;
use crate::domain::physical_plan::collections::kernels::scan_bound::item::held;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::{
    decode, facts, rows_of,
};
use crate::domain::physical_plan::collections::kernels::scan_node::item::{give_back, hold, take};
use crate::domain::physical_plan::collections::kernels::scan_roots::item::{
    copied, rooted, roots, tree_read, tree_row,
};
use crate::domain::physical_plan::collections::kernels::text_map::item::{
    RowMapRun, mapped_kinds, one_per_row, row_mapped,
};
use crate::domain::physical_plan::types::file_buffers::value::{
    ChainScratch, FileBuffers, StageScratch,
};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    Flow, KernelRun, NodeFed, NodeReaders, NodeRows, PlanNodeId, RowSource, TakenRows, TreeRoots,
};
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::names::service::name_count;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, Value};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// The cells a stage's chunk holds before its reader reads it, checked between input rows: 16 Ki cells (256 KiB of
/// 16-byte cells). A chunk keeps no more room than this after its chain's run.
pub(super) const STREAM_CHUNK_CELLS: usize = 1 << 14;

/// The rows a chain's head stage reads: a relation's columns lent in place, a tree relation's nodes (their elements lent
/// row by row), a copy of a tree relation's rows for a stage that mints while it reads, a held node's rows taken while the
/// chain reads them, or rows the engine holds for the run.
enum Head<'r> {
    /// A relation's columns.
    Facts(ColumnsView<'r>),
    /// A tree relation's nodes, read from the elements one row at a time.
    Tree(TreeRoots<'r>),
    /// A tree relation's rows copied out.
    Copied(Table),
    /// A held node and its rows and pending fault.
    Node(PlanNodeId, TakenRows),
    /// Rows the engine holds for the run.
    Held(&'r Table),
}

/// What the stages of a chain write through while its rows are pushed: the plan, the run, the worker's buffers and the
/// tail's table.
struct Push<'p, 'k, 'r> {
    /// The plan the chain's nodes are of.
    plan: &'p PhysicalPlan,
    /// The engine's rows and resources, and the matching semantics.
    run: &'k mut KernelRun<'r>,
    /// The worker's buffers: each node's decisions and the text scratch.
    buffers: &'k mut FileBuffers,
    /// The tail's rows.
    tail: &'k mut Table,
}

/// Makes `tail`'s rows by pushing the rows of the chain of streamed nodes that feeds it through each stage in turn, holding
/// only `tail`'s rows and pending fault. A node of the chain already held in `buffers` ends the chain and is read as a source.
///
/// # Errors
///
/// The chain's first hard fault in the held plan's order: a refused relation, a column past a stage's input rows.
pub(in crate::domain::physical_plan::service::run) fn streamed(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    tail: PlanNodeId,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    let mut chain = buffers.stream.levels.pop().unwrap_or_default();
    let answered = chained(plan, run, tail, buffers, &mut chain);
    for stage in &mut chain.stages {
        settled(stage);
    }
    buffers.stream.levels.push(chain);
    answered
}

/// Empties a stage's chunk after its chain's run, keeping no more room than the bound.
fn settled(stage: &mut StageScratch) {
    stage.out.kinds.clear();
    stage.out.cells.clear();
    stage.out.count = RowCount(0);
    if stage.out.cells.capacity() > STREAM_CHUNK_CELLS {
        stage.out.cells.shrink_to(STREAM_CHUNK_CELLS);
    }
    stage.fault = None;
}

/// The node at `node`.
fn physical(
    plan: &PhysicalPlan,
    node: PlanNodeId,
) -> Result<
    &crate::domain::physical_plan::types::physical_plan::value::PhysicalNode,
    PortableOperatorFault,
> {
    plan.nodes
        .get(usize::try_from(node.0).unwrap_or(usize::MAX))
        .ok_or(PortableOperatorFault::WrongOperation)
}

/// The first source of `node`: what a stage reads row by row.
fn first_source(plan: &PhysicalPlan, node: PlanNodeId) -> Result<RowSource, PortableOperatorFault> {
    physical(plan, node)?
        .lowering
        .sources()
        .next()
        .ok_or(PortableOperatorFault::WrongOperation)
}

/// Fills `chain` with the nodes that stream into `tail`, from the head stage to `tail`: each first source that streams and
/// holds no rows in this file.
fn walked(
    plan: &PhysicalPlan,
    tail: PlanNodeId,
    buffers: &FileBuffers,
    chain: &mut Vec<PlanNodeId>,
) -> Result<(), PortableOperatorFault> {
    chain.clear();
    let mut at = tail;
    loop {
        chain.push(at);
        match first_source(plan, at)? {
            RowSource::Node(read)
                if physical(plan, read)?.flow == Flow::Streamed
                    && buffers.rows(read) == NodeRows::Absent =>
            {
                at = read;
            }
            RowSource::Node(_)
            | RowSource::Facts(_)
            | RowSource::Tree(_)
            | RowSource::Frontier(_)
            | RowSource::Observation(_) => break,
        }
    }
    chain.reverse();
    Ok(())
}

/// Reads the head stage's source: `reader` reads it, and copies a tree relation's rows when it mints while it reads.
fn head<'r>(
    run: &KernelRun<'r>,
    reader: PlanNodeId,
    lowering: &Lowering,
    source: RowSource,
    buffers: &mut FileBuffers,
) -> Result<Head<'r>, PortableOperatorFault> {
    let inputs = run.inputs;
    Ok(match source {
        RowSource::Facts(relation) => Head::Facts(facts(inputs, relation)?),
        RowSource::Tree(relation) => match lowering {
            Lowering::TextMap { .. } => Head::Copied(copied(&roots(
                inputs,
                run.resources.tree(),
                reader,
                relation,
            ))),
            _ => Head::Tree(rooted(inputs, reader, relation)),
        },
        RowSource::Node(read) => Head::Node(read, take(buffers, read)?),
        // A withheld input faults where it is first read, never as empty rows.
        RowSource::Frontier(place) => Head::Held(held(inputs.frontier(place))?),
        RowSource::Observation(place) => Head::Held(held(inputs.observation(place))?),
    })
}

/// The head's column kinds and row count.
fn shape<'h>(head: &'h Head<'_>) -> (&'h [CellKind], RowCount) {
    match head {
        Head::Facts(view) => (view.kinds, rows_of(view)),
        Head::Tree(roots) => (roots.kinds, RowCount(roots.candidates.len())),
        Head::Copied(table) | Head::Node(_, TakenRows { rows: table, .. }) => {
            (&table.kinds, table.count)
        }
        Head::Held(table) => (&table.kinds, table.count),
    }
}

/// Runs the chain into `tail` and holds its rows; gives the head's rows back whatever it answers.
fn chained(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    tail: PlanNodeId,
    buffers: &mut FileBuffers,
    chain: &mut ChainScratch,
) -> Result<(), PortableOperatorFault> {
    walked(plan, tail, buffers, &mut chain.chain)?;
    let length = chain.chain.len();
    if chain.stages.len() < length {
        chain.stages.resize_with(length, StageScratch::default);
    }
    let head_node = *chain
        .chain
        .first()
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let source = first_source(plan, head_node)?;
    super::super::service::ready(plan, run, source, buffers)?;
    let head = head(
        run,
        head_node,
        &physical(plan, head_node)?.lowering,
        source,
        buffers,
    )?;
    let mut output = Table::default();
    let answered = pushed_all(plan, run, buffers, chain, &head, &mut output);
    let pending = match &head {
        Head::Node(_, taken) => taken.pending.clone(),
        Head::Facts(_) | Head::Tree(_) | Head::Copied(_) | Head::Held(_) => None,
    };
    if let Head::Node(read, taken) = head {
        give_back(plan, buffers, read, taken, NodeReaders(1));
    }
    let fault = answered?;
    hold(plan, buffers, tail, output, fault.or(pending));
    Ok(())
}

/// Readies every stage of `chain` in order and pushes the head's rows through them into `output`; answers the fault of the
/// last stage that stopped at one, the one the held plan's tail raises.
///
/// # Errors
///
/// A stage's hard fault: a column past its input rows.
fn pushed_all(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    buffers: &mut FileBuffers,
    chain: &mut ChainScratch,
    head: &Head<'_>,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let ChainScratch {
        chain: nodes,
        row,
        stages,
    } = chain;
    let length = nodes.len();
    let stages = stages
        .get_mut(..length)
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let (head_kinds, head_rows) = shape(head);
    prepared(plan, run, buffers, nodes, stages, head_kinds, output)?;
    // A stage that writes one row per row it reads, alone on its head, has its table sized once.
    if let [node] = nodes.as_slice()
        && let Lowering::TextMap { map, .. } = &physical(plan, *node)?.lowering
        && one_per_row(map)
    {
        output
            .cells
            .reserve_exact(head_rows.0.saturating_mul(output.kinds.len()));
    }
    let mut push = Push {
        plan,
        run,
        buffers,
        tail: output,
    };
    for at in 0..head_rows.0 {
        if stages.iter().any(|stage| stage.fault.is_some()) {
            break;
        }
        let cells: &[Value] = match head {
            Head::Facts(view) => {
                decode(view, RowIndex(at), row);
                row
            }
            Head::Tree(roots) => {
                tree_row(
                    &tree_read(*roots, push.run.resources.tree()),
                    RowIndex(at),
                    row,
                );
                row
            }
            Head::Copied(table) | Head::Node(_, TakenRows { rows: table, .. }) => {
                row_at(table, RowIndex(at))
            }
            Head::Held(table) => row_at(table, RowIndex(at)),
        };
        pushed(&mut push, nodes, stages, head_kinds, cells)?;
    }
    // The chunks left: each stage's rows to its reader, the head stage's first.
    for at in 0..length {
        let (Some(readers), Some(stages)) = (nodes.get(at + 1..), stages.get_mut(at..)) else {
            break;
        };
        if let Some((stage, after)) = stages.split_first_mut() {
            flushed(&mut push, readers, stage, after)?;
        }
    }
    let tail_fed = if length == 1 {
        head_rows
    } else {
        stages.last().map_or(RowCount(0), |stage| stage.read)
    };
    if let Some(fed) = nodes.last().and_then(|tail| {
        push.buffers
            .fed
            .get_mut(usize::try_from(tail.0).unwrap_or(usize::MAX))
    }) {
        *fed = NodeFed([tail_fed, RowCount(0), RowCount(0)]);
    }
    Ok(stages.iter_mut().rev().find_map(|stage| stage.fault.take()))
}

/// The cells of `row` of `table`.
fn row_at(table: &Table, row: RowIndex) -> &[Value] {
    let width = table.kinds.len().max(1);
    table
        .cells
        .get(row.0 * width..row.0 * width + width)
        .unwrap_or_default()
}

/// Readies each stage in chain order before any row is read, as the held plan readies each node before its rows: its
/// output kinds from its input's, a filter's memo for the file's names, an extension's values, a distinct tail's repeats.
/// A stage whose preparation faults stops before its first row; the stages after it are readied over its empty kinds.
///
/// # Errors
///
/// A stage's shape fault: a kept or projected column past its input rows.
fn prepared(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    buffers: &mut FileBuffers,
    nodes: &[PlanNodeId],
    stages: &mut [StageScratch],
    head_kinds: &[CellKind],
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let names =
        crate::domain::comparing::types::NameCount(name_count(run.resources.texts().symbols).0);
    let last = nodes.len().saturating_sub(1);
    for (at, node) in nodes.iter().enumerate() {
        let lowering = &physical(plan, *node)?.lowering;
        let (before, from) = stages.split_at_mut(at);
        let input: &[CellKind] = before.last().map_or(head_kinds, |stage| &stage.out.kinds);
        let stage = from
            .first_mut()
            .ok_or(PortableOperatorFault::WrongOperation)?;
        stage.out.cells.clear();
        stage.out.count = RowCount(0);
        stage.last = 0;
        stage.read = RowCount(0);
        stage.fault = None;
        let (kinds, prepared) = stage_prepared(run, buffers, *node, lowering, input, names, stage)?;
        stage.fault = prepared.fault;
        if at == last {
            output.kinds = kinds;
        } else {
            stage.out.kinds = kinds;
        }
    }
    Ok(())
}

/// The output kinds of one stage, `node` lowered to `lowering`, over input rows of `input`, and what its preparation left: in
/// the held kernel's own order, an extension's values resolved before its kinds, its kinds left empty when they fault.
///
/// # Errors
///
/// A stage's shape fault: a kept or projected column past its input rows.
fn stage_prepared(
    run: &mut KernelRun<'_>,
    buffers: &mut FileBuffers,
    node: PlanNodeId,
    lowering: &Lowering,
    input: &[CellKind],
    names: crate::domain::comparing::types::NameCount,
    stage: &mut StageScratch,
) -> Result<(Vec<CellKind>, StagePrepared), PortableOperatorFault> {
    Ok(match lowering {
        Lowering::Filter { select, .. } => {
            let kinds = kept_kinds(input, &select.kept)?;
            if let Some(decisions) = buffers
                .decisions
                .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
            {
                readied(select, decisions, names);
            }
            (kinds, StagePrepared::default())
        }
        Lowering::TextMap { map, .. } => (mapped_kinds(map, input), StagePrepared::default()),
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
            emptied(&mut stage.repeats);
            (kinds, StagePrepared::default())
        }
        // An extension's values are made before its rows are read: making a name changes the file's names, and reading
        // reads them.
        Lowering::Extend { extension, .. } => match resolved(
            &extension.values,
            &extension.bindings,
            run.resources,
            &mut stage.values,
        ) {
            Ok(()) => (
                input
                    .iter()
                    .copied()
                    .chain(stage.values.iter().map(|value| kind_of(*value)))
                    .collect(),
                StagePrepared::default(),
            ),
            Err(fault) => (Vec::new(), StagePrepared { fault: Some(fault) }),
        },
        _ => return Err(PortableOperatorFault::WrongOperation),
    })
}

/// Pushes one input row, of `kinds`, through the first stage of `stages` (the stage of `nodes`' first node): its rows are
/// written to its chunk, or the tail's table for the last stage, and a chunk that reached the bound is handed on. A stage
/// that faults hands on the rows it wrote before the fault and reads no row after it.
///
/// # Errors
///
/// A stage's hard fault.
fn pushed(
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
    if stage.fault.is_some() {
        return Ok(());
    }
    stage.read = RowCount(stage.read.0.saturating_add(1));
    let lowering = &physical(push.plan, *node)?.lowering;
    let place = usize::try_from(node.0).unwrap_or(usize::MAX);
    let out: &mut Table = if after.is_empty() {
        &mut *push.tail
    } else {
        &mut stage.out
    };
    stage.last = out.cells.len();
    let fault = match lowering {
        Lowering::Filter { select, .. } => {
            let decisions = push
                .buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            let texts: Texts<'_> = push.run.resources.texts();
            let kernel = filter(select, kinds, texts);
            keep(
                kernel,
                push.run.matcher,
                cells,
                decisions,
                &mut push.buffers.written,
                out,
            )
            .err()
        }
        Lowering::TextMap { map, .. } => {
            let decisions = push
                .buffers
                .decisions
                .get_mut(place)
                .ok_or(PortableOperatorFault::WrongOperation)?;
            row_mapped(
                map,
                cells,
                &mut RowMapRun {
                    resources: &mut *push.run.resources,
                    matcher: push.run.matcher,
                    decisions,
                    scratch: &mut push.buffers.scratch,
                    places: &mut push.buffers.places,
                    output: out,
                },
            )
            .err()
        }
        Lowering::Project { project, .. } => projected(
            cells,
            &project.columns,
            project.distinct,
            &mut stage.repeats,
            out,
        )
        .err(),
        Lowering::Extend { .. } => {
            extended(cells, &stage.values, &mut push.buffers.written, out);
            None
        }
        _ => return Err(PortableOperatorFault::WrongOperation),
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
/// memo first grows to the names the chunk's stage minted.
///
/// # Errors
///
/// A stage's hard fault.
fn flushed(
    push: &mut Push<'_, '_, '_>,
    nodes: &[PlanNodeId],
    stage: &mut StageScratch,
    after: &mut [StageScratch],
) -> Result<(), PortableOperatorFault> {
    let Some(reader) = nodes.first() else {
        return Ok(());
    };
    #[cfg(test)]
    if stage.last >= STREAM_CHUNK_CELLS {
        push.buffers.chunks.over = true;
    }
    if let Lowering::Filter { select, .. } = &physical(push.plan, *reader)?.lowering
        && let Some(decisions) = push
            .buffers
            .decisions
            .get_mut(usize::try_from(reader.0).unwrap_or(usize::MAX))
    {
        let names = crate::domain::comparing::types::NameCount(
            name_count(push.run.resources.texts().symbols).0,
        );
        grown(select, decisions, names);
    }
    for at in 0..stage.out.count.0 {
        if after.first().is_none_or(|next| next.fault.is_some()) {
            break;
        }
        pushed(
            push,
            nodes,
            after,
            &stage.out.kinds,
            row_at(&stage.out, RowIndex(at)),
        )?;
    }
    stage.out.cells.clear();
    stage.out.count = RowCount(0);
    stage.last = 0;
    Ok(())
}
