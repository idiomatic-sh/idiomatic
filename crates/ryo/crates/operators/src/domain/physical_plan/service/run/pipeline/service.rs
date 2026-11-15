//! The runner's chain of streamed stages: the rows of the stages that feed a node, pushed through each stage in turn in
//! bounded chunks, only the node's own rows held. Each stage keeps the rows it wrote before its first fault and stops there;
//! its reader reads every row before that fault, so the earliest row's fault wins whatever the chunking (D15). A hard fault
//! is the held plan's first: the first stage, in chain order, whose own run raises one.
use super::super::types::ChainRows;
use super::push::service::pushed_all;
use super::sources::service::{
    given_back, pending_of, physical, reading, resolved_rows, right_of, shape, walked,
};
use super::stages::service::stage_prepared;
use crate::domain::comparing::types::NameCount;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::hash_index::item::emptied;
use crate::domain::physical_plan::collections::kernels::scan_node::item::hold;
use crate::domain::physical_plan::service::indexes::service::{closed, opened};
use crate::domain::physical_plan::types::file_buffers::value::{
    ChainScratch, FileBuffers, StageScratch,
};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::{PhysicalNode, PhysicalPlan};
use crate::domain::physical_plan::types::{KernelRun, PlanNodeId, ProbeIndex, RowSource};
use ryo_language::domain::names::service::name_count;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::rows::types::{RowCount, Table};

/// The cells a stage's chunk holds before its reader reads it, checked between input rows: 16 Ki cells (256 KiB of
/// 16-byte cells). A chunk keeps no more room than this after its chain's run.
pub(super) const STREAM_CHUNK_CELLS: usize = 1 << 14;

/// What the stages of a chain write through while its rows are pushed: the plan, the run, the worker's buffers, the rows
/// the chain reads besides its stages' own, the tail's table, and the chain's first hard fault.
pub(super) struct Push<'p, 'k, 'r> {
    /// The plan the chain's nodes are of.
    pub(super) plan: &'p PhysicalPlan,
    /// The engine's rows and resources, and the matching semantics.
    pub(super) run: &'k mut KernelRun<'r>,
    /// The worker's buffers: each node's decisions, the text scratch and the shared indexes.
    pub(super) buffers: &'k mut FileBuffers,
    /// The head stage's source.
    pub(super) head: &'k ChainRows<'r>,
    /// Each probe's right side, in chain order.
    pub(super) rights: &'k [ChainRows<'r>],
    /// The tail's rows.
    pub(super) tail: &'k mut Table,
    /// How many stages the chain has.
    pub(super) length: usize,
    /// The first hard fault of the chain in the held plan's order: the place of its stage, and the fault.
    pub(super) hard: Option<(usize, PortableOperatorFault)>,
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

/// Empties a stage after its chain's run, keeping no more room in its tables than the bound.
fn settled(stage: &mut StageScratch) {
    for table in [&mut stage.out, &mut stage.own.kept] {
        table.kinds.clear();
        table.cells.clear();
        table.count = RowCount(0);
        if table.cells.capacity() > STREAM_CHUNK_CELLS {
            table.cells.shrink_to(STREAM_CHUNK_CELLS);
        }
    }
    emptied(&mut stage.own.index);
    stage.fault = None;
}

/// Records `fault` as the chain's hard fault when it comes before the one recorded: at an earlier stage.
pub(super) fn hard_at(
    hard: &mut Option<(usize, PortableOperatorFault)>,
    at: usize,
    fault: PortableOperatorFault,
) {
    if hard.as_ref().is_none_or(|(first, _)| at < *first) {
        *hard = Some((at, fault));
    }
}

/// Runs the chain into `tail` and holds its rows. In the held plan's order, before any row: the head's source and each
/// probe's right side are readied (made when they are held nodes not made yet), then read, then each stage is prepared.
/// Every reading is given back, and every shared index a probe opened closed, whatever the chain answers.
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
    let ChainScratch {
        chain: nodes,
        row,
        stages,
    } = chain;
    let stages = stages
        .get_mut(..length)
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let head_node = *nodes.first().ok_or(PortableOperatorFault::WrongOperation)?;
    let head_lowering = &physical(plan, head_node)?.lowering;
    let source = head_lowering
        .sources()
        .next()
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let mut hard = None;
    // Every source is readied before any is read, so a right side's own chain never finds a source this chain has taken.
    super::super::service::ready(plan, run, source, buffers)?;
    let mut limit = length;
    for (at, node) in nodes.iter().enumerate() {
        if let Some(right) = right_of(&physical(plan, *node)?.lowering)
            && let Err(fault) = super::super::service::ready(plan, run, right, buffers)
        {
            hard_at(&mut hard, at, fault);
            limit = at;
            break;
        }
    }
    // A right side of the head stage is readied before the head's source is read.
    if let Some((0, fault)) = hard {
        return Err(fault);
    }
    let head = reading(
        run,
        head_node,
        matches!(head_lowering, Lowering::TextMap { .. }),
        source,
        buffers,
    )?;
    let mut rights: Vec<ChainRows<'_>> = Vec::new();
    for (at, node) in nodes.iter().enumerate().take(limit) {
        let Some(right) = right_of(&physical(plan, *node)?.lowering) else {
            continue;
        };
        let earlier = rights.iter().position(|taken| {
            matches!((taken, right), (ChainRows::Node(read, _), RowSource::Node(node)) if *read == node)
        });
        let rows = match (source, earlier) {
            (RowSource::Node(head), _) if right == RowSource::Node(head) => Ok(ChainRows::Head),
            (_, Some(earlier)) => Ok(ChainRows::Right(earlier)),
            _ => reading(run, *node, false, right, buffers),
        };
        match rows {
            Ok(rows) => {
                if let Some(stage) = stages.get_mut(at) {
                    stage.own.right = rights.len();
                }
                rights.push(rows);
            }
            Err(fault) => {
                hard_at(&mut hard, at, fault);
                limit = at;
                break;
            }
        }
    }
    let mut output = Table::default();
    let mut push = Push {
        plan,
        run,
        buffers,
        head: &head,
        rights: &rights,
        tail: &mut output,
        length,
        hard,
    };
    let reached = prepared(&mut push, nodes, stages, limit);
    let opened_stages = reached.as_ref().map_or(0, |reached| *reached);
    let answered = reached.and_then(|_| pushed_all(&mut push, nodes, row, stages));
    let hard = push.hard.take();
    for node in nodes.iter().take(opened_stages) {
        if let Ok(PhysicalNode {
            lowering:
                Lowering::Probe {
                    index: ProbeIndex::Shared(at),
                    ..
                },
            ..
        }) = physical(plan, *node)
        {
            closed(buffers, *at);
        }
    }
    let pending = pending_of(&head);
    given_back(plan, buffers, head, rights);
    if let Some((_, fault)) = hard {
        return Err(fault);
    }
    let fault = answered?;
    hold(plan, buffers, tail, output, fault.or(pending));
    Ok(())
}

/// Prepares the first `limit` stages in chain order before any row is read, as the held plan readies each node before its
/// rows: each one's output kinds from its input's, through `stage_prepared`. A stage whose preparation faults stops before
/// its first row, and the stages after it are prepared over its empty kinds; a stage's hard fault stops the preparation
/// there, and it and every stage after it read no row. A probe of a shared index opens it. Answers how many stages it
/// reached, the one that raised a hard fault included.
///
/// # Errors
///
/// A stage the plan does not hold; a stage's hard fault is the chain's, recorded at its place.
fn prepared(
    push: &mut Push<'_, '_, '_>,
    nodes: &[PlanNodeId],
    stages: &mut [StageScratch],
    limit: usize,
) -> Result<usize, PortableOperatorFault> {
    let names = NameCount(name_count(push.run.resources.texts().symbols).0);
    let (head_kinds, _) = shape(push.head);
    let last = nodes.len().saturating_sub(1);
    for (at, node) in nodes.iter().enumerate() {
        let (before, from) = stages.split_at_mut(at);
        let stage = from
            .first_mut()
            .ok_or(PortableOperatorFault::WrongOperation)?;
        stage.out.cells.clear();
        stage.out.count = RowCount(0);
        stage.last = 0;
        stage.read = RowCount(0);
        stage.fault = None;
        if at >= limit {
            // Not reached: an earlier stage's source or preparation raised the chain's hard fault.
            stage.fault = Some(PortableOperatorFault::NotRun);
            continue;
        }
        let lowering = &physical(push.plan, *node)?.lowering;
        if let Lowering::Probe {
            index: ProbeIndex::Shared(place),
            ..
        } = lowering
        {
            opened(push.plan, push.buffers, *place);
        }
        let input: &[CellKind] = before.last().map_or(head_kinds, |stage| &stage.out.kinds);
        let right = right_of(lowering).and_then(|_| {
            push.rights
                .get(stage.own.right)
                .map(|rows| resolved_rows(rows, push.head, push.rights))
        });
        match stage_prepared(
            push.run,
            push.buffers,
            (*node, lowering),
            input,
            names,
            &mut stage.own,
            right,
        ) {
            Ok((kinds, prepared)) => {
                stage.fault = prepared.fault;
                if at == last {
                    push.tail.kinds = kinds;
                } else {
                    stage.out.kinds = kinds;
                }
            }
            Err(fault) => {
                hard_at(&mut push.hard, at, fault.clone());
                stage.fault = Some(fault);
                for later in from.iter_mut().skip(1) {
                    later.fault = Some(PortableOperatorFault::NotRun);
                }
                return Ok(at + 1);
            }
        }
    }
    Ok(limit.min(nodes.len()))
}
