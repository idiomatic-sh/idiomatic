//! The runner's chain of streamed stages: the rows of the stages that feed a node, pushed through each stage in turn in
//! bounded chunks, only the node's own rows held. Each stage keeps the rows it wrote before its first fault and stops there;
//! its reader reads every row before that fault, so the earliest row's fault wins whatever the chunking (D15). A hard fault
//! is the held plan's first: the first stage, in chain order, whose own run raises one.
use super::super::types::{ChainFault, ChainRows, ChainStage, Push, Readied};
use super::push::service::{first_fault, pushed_all};
use super::sources::service::{
    given_back, pending_of, physical, reading, resolved_rows, right_of, rows_kinds, walked,
};
use super::stages::service::stage_prepared;
use crate::domain::comparing::types::NameCount;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::scan_node::item::{hold, take};
use crate::domain::physical_plan::service::indexes::service::{closed, opened};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::stream_scratch::value::{
    ChainScratch, RightPlace, StageScratch,
};
use crate::domain::physical_plan::types::{
    CellStart, KernelRun, PlanNodeId, ProbeIndex, RowSource,
};
use ryo_language::domain::names::service::name_count;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::rows::types::RowCount;

/// Makes `tail`'s rows by pushing the rows of the chain of streamed nodes that feeds it through each stage in turn, holding
/// only `tail`'s rows and pending fault. A node of the chain already held in `buffers` ends the chain and is read as a source;
/// `ready` makes each held source that holds no rows yet.
///
/// # Errors
///
/// The chain's first hard fault in the held plan's order: a refused relation, a column past a stage's input rows.
pub(in crate::domain::physical_plan::service::run) fn streamed(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    tail: PlanNodeId,
    buffers: &mut FileBuffers,
    ready: Readied,
) -> Result<(), PortableOperatorFault> {
    let mut chain = buffers.stream.levels.pop().unwrap_or_default();
    let answered = chained(plan, run, tail, buffers, &mut chain, ready);
    for stage in &mut chain.stages {
        stage.settled();
    }
    buffers.stream.levels.push(chain);
    answered
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
    ready: Readied,
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
    let head = ChainStage {
        node: *nodes.first().ok_or(PortableOperatorFault::WrongOperation)?,
        lowering: &physical(
            plan,
            *nodes.first().ok_or(PortableOperatorFault::WrongOperation)?,
        )?
        .lowering,
    };
    let source = head
        .lowering
        .sources()
        .next()
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let mut hard = None;
    // The held plan's order: the head stage readies its source and its right side before it reads either, and each later
    // probe readies and reads its right side after the stages before it. Reading a relation demands it of the facts, so a
    // relation is read in that order; a held node's rows are taken only once every source is readied, so a right side's own
    // chain never finds a source this chain has taken.
    ready(plan, run, source, buffers)?;
    if let Some(right) = right_of(head.lowering) {
        ready(plan, run, right, buffers)?;
    }
    let head_read = match source {
        RowSource::Node(_) => None,
        RowSource::Facts(_)
        | RowSource::Tree(_)
        | RowSource::Frontier(_)
        | RowSource::Observation(_) => Some(reading(run, head, source, buffers)?),
    };
    let mut rights: Vec<ChainRows<'_>> = Vec::new();
    for (node, stage) in nodes.iter().zip(stages.iter_mut()) {
        let lowering = &physical(plan, *node)?.lowering;
        let Some(right) = right_of(lowering) else {
            continue;
        };
        if *node != head.node
            && let Err(fault) = ready(plan, run, right, buffers)
        {
            first_fault(&mut hard, ChainFault { node: *node, fault });
            break;
        }
        let rows = match right {
            RowSource::Node(read) => Ok(ChainRows::Untaken(read)),
            RowSource::Facts(_)
            | RowSource::Tree(_)
            | RowSource::Frontier(_)
            | RowSource::Observation(_) => reading(
                run,
                ChainStage {
                    node: *node,
                    lowering,
                },
                right,
                buffers,
            ),
        };
        match rows {
            Ok(rows) => {
                stage.own.right = RightPlace(rights.len());
                rights.push(rows);
            }
            Err(fault) => {
                first_fault(&mut hard, ChainFault { node: *node, fault });
                break;
            }
        }
    }
    // Every source readied: the held nodes' rows are taken, each once, however many stages read it.
    let head_rows = match head_read {
        Some(rows) => rows,
        None => reading(run, head, source, buffers)?,
    };
    for at in 0..rights.len() {
        let Some(ChainRows::Untaken(read)) = rights.get(at) else {
            continue;
        };
        let read = *read;
        let earlier = rights
            .iter()
            .take(at)
            .position(|taken| matches!(taken, ChainRows::Node(node, _) if *node == read));
        let rows = match (source, earlier) {
            (RowSource::Node(head_source), _) if head_source == read => ChainRows::Head,
            (_, Some(earlier)) => ChainRows::Right(RightPlace(earlier)),
            _ => ChainRows::Node(read, take(buffers, read)?),
        };
        if let Some(slot) = rights.get_mut(at) {
            *slot = rows;
        }
    }
    // The tail's table: one the worker's pool kept, its room reused.
    let mut output = buffers.pool.fresh();
    let mut push = Push {
        plan,
        run,
        buffers,
        head: &head_rows,
        rights: &rights,
        tail: &mut output,
        hard,
    };
    prepared(&mut push, nodes, stages)?;
    let answered = pushed_all(&mut push, nodes, row, stages);
    let hard = push.hard.take();
    for stage in stages.iter_mut() {
        if let Some(at) = stage.own.shared.take() {
            closed(buffers, at);
        }
    }
    let pending = pending_of(&head_rows);
    given_back(buffers, head_rows, rights);
    if let Some(first) = hard {
        return Err(first.fault);
    }
    let fault = answered?;
    hold(plan, buffers, tail, output, fault.or(pending));
    Ok(())
}

/// Prepares each stage in chain order before any row is read, as the held plan readies each node before its rows: each one's
/// output kinds from its input's, through `stage_prepared`. A stage whose preparation faults stops before its first row, and
/// the stages after it are prepared over its empty kinds. A stage at or past the chain's hard fault is not reached: it, and
/// every stage after it, reads no row. A probe of a shared index opens it.
///
/// # Errors
///
/// A stage the plan does not hold; a stage's hard fault is the chain's, recorded at its node.
fn prepared(
    push: &mut Push<'_, '_, '_>,
    nodes: &[PlanNodeId],
    stages: &mut [StageScratch],
) -> Result<(), PortableOperatorFault> {
    let names = NameCount(name_count(push.run.resources.texts().symbols).0);
    let head_kinds = rows_kinds(push.head);
    let last = nodes.len().saturating_sub(1);
    for (at, node) in nodes.iter().enumerate() {
        let (before, from) = stages.split_at_mut(at);
        let stage = from
            .first_mut()
            .ok_or(PortableOperatorFault::WrongOperation)?;
        stage.out.cells.clear();
        stage.out.count = RowCount(0);
        stage.last = CellStart(0);
        stage.read = RowCount(0);
        stage.fault = None;
        if push.hard.as_ref().is_some_and(|first| first.node <= *node) {
            // Not reached: an earlier source or stage raised the chain's hard fault, or this stage's right side did.
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
            stage.own.shared = Some(*place);
        }
        let input: &[CellKind] = before.last().map_or(head_kinds, |stage| &stage.out.kinds);
        let right = right_of(lowering).and_then(|_| {
            push.rights
                .get(stage.own.right.0)
                .map(|rows| resolved_rows(rows, push.head, push.rights))
        });
        match stage_prepared(
            push.run,
            push.buffers,
            ChainStage {
                node: *node,
                lowering,
            },
            input,
            names,
            &mut stage.own,
            right,
        ) {
            Ok(prepared) => {
                stage.fault = prepared.fault;
                if at == last {
                    push.tail.kinds = prepared.kinds;
                } else {
                    stage.out.kinds = prepared.kinds;
                }
            }
            Err(fault) => {
                first_fault(
                    &mut push.hard,
                    ChainFault {
                        node: *node,
                        fault: fault.clone(),
                    },
                );
                stage.fault = Some(fault);
            }
        }
    }
    Ok(())
}
