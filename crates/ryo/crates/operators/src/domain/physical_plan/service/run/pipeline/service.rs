//! The runner's chain of streamed stages: the rows of the stages that feed a node, pushed through each stage in turn in
//! bounded chunks, only the node's own rows held. Each stage keeps the rows it wrote before its first fault and stops there;
//! its reader reads every row before that fault, so the earliest row's fault wins whatever the chunking (D15). A hard fault
//! is the held plan's first: the first stage, in chain order, whose own run raises one.
use super::super::types::{ChainFault, ChainRows, ChainStage, Prepare, Push, Readied};
use super::push::service::pushed_all;
use super::sources::service::{
    given_back, pending_of, physical, reading, resolved_rows, right_of, rows_kinds, walked,
};
use super::stages::service::stage_prepared;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::scan_node::item::{hold, take};
use crate::domain::physical_plan::service::indexes::service::{closed, opened};
use crate::domain::physical_plan::service::run::counting::service::{timed, timed_to};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::stream_scratch::value::{
    ChainScratch, RightPlace, StagePlace, StageScratch,
};
use crate::domain::physical_plan::types::{KernelRun, PlanNodeId, ProbeIndex, RowSource};
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
    // Spike (ryo-fmx3): demand-driven, the chain's head source is found and settled before any of the chain's own room is
    // taken: a chain over a node with no row is marked empty at once.
    if crate::domain::physical_plan::service::run::service::lazy() {
        use crate::domain::physical_plan::service::run::service::{Known, held_empty, known, marked_empty, settled};
        let mut at = tail;
        let source = loop {
            let lowering = &physical(plan, at)?.lowering;
            let Some(first) = lowering.sources().next() else {
                return Err(PortableOperatorFault::WrongOperation);
            };
            if at != tail && matches!(lowering, Lowering::Probe { .. }) {
                break first;
            }
            if matches!(lowering, Lowering::Probe { .. }) {
                break first;
            }
            match first {
                RowSource::Node(read)
                    if physical(plan, read)?.flow == crate::domain::physical_plan::types::NodeFlow::Streamed
                        && buffers.rows(read) == crate::domain::physical_plan::types::NodeRows::Absent
                        && known(buffers, read.0) != Known::Empty =>
                {
                    at = read;
                }
                other => break other,
            }
        };
        settled(plan, run, source, buffers)?;
        if held_empty(buffers, source) {
            let origin = crate::domain::physical_plan::service::run::service::origin_of(buffers, source);
            marked_empty(plan, buffers, tail, origin);
            return Ok(());
        }
        // Spike (ryo-fmx3): a partitioned sibling at the chain's head whose bucket and the shared rest hold no row holds none,
        // so neither does the chain: the head is marked empty, as having emptied itself, and the tail after it, no row read.
        if crate::spike::fanout()
            && crate::domain::physical_plan::collections::kernels::partition::item::sibling_rows(plan, run, at, source, buffers, false)
                == Some(true)
        {
            marked_empty(plan, buffers, at, at.0);
            if at != tail {
                let origin = crate::domain::physical_plan::service::run::service::origin_of(buffers, RowSource::Node(at));
                marked_empty(plan, buffers, tail, origin);
            }
            return Ok(());
        }
    }
    let mut chain = buffers.stream.levels.pop().unwrap_or_default();
    let answered = chained(plan, run, tail, buffers, &mut chain, ready);
    chain.settled();
    chain.release_private(&mut buffers.pools);
    buffers.stream.levels.push(chain);
    answered
}

/// Runs the chain into `tail` and holds its rows. Before any row, stage by stage in chain order, it does what the held plan
/// does as it makes each node, and nothing past the first stage that fails: the head stage readies its source and its right
/// side, then reads them; each later probe readies its right side, then reads it; then the stage is prepared. A source is
/// asked of the inputs only where the held plan asks it, so a stage that fails first leaves every later source unasked.
/// While a right side is readied, the held nodes the chain has taken are lent back, so the right side's own chain finds them
/// as the held plan's would. Every reading is given back, and every shared index a probe opened closed, whatever the chain
/// answers.
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
        head: head_chunk,
        stages,
    } = chain;
    let stages = stages
        .get_mut(..length)
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let head_node = *nodes.first().ok_or(PortableOperatorFault::WrongOperation)?;
    let source = physical(plan, head_node)?
        .lowering
        .sources()
        .next()
        .ok_or(PortableOperatorFault::WrongOperation)?;
    for stage in stages.iter_mut() {
        stage.out.cells.clear();
        stage.out.count = RowCount(0);
        stage.read = RowCount(0);
        // Not reached until it is prepared: a stage past the chain's first failure reads no row.
        stage.fault = Some(PortableOperatorFault::NotRun);
    }
    let mut hard = None;
    let mut head_rows: Option<ChainRows<'_>> = None;
    let mut rights: Vec<ChainRows<'_>> = Vec::new();
    // The tail's table: one the worker's pool kept, its room reused.
    let mut output = buffers.pool.fresh();
    for (at, node) in nodes.iter().enumerate() {
        let stage = ChainStage {
            node: *node,
            lowering: &physical(plan, *node)?.lowering,
        };
        if at == 0 {
            // Spike (ryo-fmx3): every streamed stage answers no row over no row, so a chain whose head reads a node with no
            // row holds none: the tail is marked empty, no table held, its right sides never readied.
            if crate::domain::physical_plan::service::run::service::lazy() {
                crate::domain::physical_plan::service::run::service::settled(plan, run, source, buffers)?;
                if crate::domain::physical_plan::service::run::service::held_empty(buffers, source) {
                    buffers.pool.pooled(output, crate::domain::kept_room::types::PoolSize(crate::domain::kept_room::constants::POOL_TABLES));
                    let origin = crate::domain::physical_plan::service::run::service::origin_of(buffers, source);
                    crate::domain::physical_plan::service::run::service::marked_empty(plan, buffers, tail, origin);
                    return Ok(());
                }
            }
            ready(plan, run, source, buffers)?;
        }
        let right = right_of(stage.lowering);
        if let Some(right) = right {
            for rows in head_rows.iter_mut().chain(rights.iter_mut()) {
                lent(rows, buffers);
            }
            // Spike (ryo-fmx3): an inner or semi probe at the chain's head over a right side with no row answers no row; the
            // right side is settled before its table is asked for, and nothing is read yet.
            if at == 0
                && crate::domain::physical_plan::service::run::service::lazy()
                && matches!(
                    stage.lowering,
                    Lowering::Probe { probe, .. } if matches!(
                        probe.mode,
                        crate::domain::equality_join::types::ProbeMode::Inner
                            | crate::domain::equality_join::types::ProbeMode::Semi
                    )
                )
            {
                crate::domain::physical_plan::service::run::service::settled(plan, run, right, buffers)?;
                if crate::domain::physical_plan::service::run::service::held_empty(buffers, right) {
                    buffers.pool.pooled(output, crate::domain::kept_room::types::PoolSize(crate::domain::kept_room::constants::POOL_TABLES));
                    let origin = crate::domain::physical_plan::service::run::service::origin_of(buffers, right);
                    crate::domain::physical_plan::service::run::service::marked_empty(plan, buffers, tail, origin);
                    return Ok(());
                }
            }
            let readied = ready(plan, run, right, buffers);
            for rows in head_rows.iter_mut().chain(rights.iter_mut()) {
                retaken(rows, buffers)?;
            }
            if let Err(fault) = readied {
                hard = Some(ChainFault { node: *node, fault });
                break;
            }
        }
        if at == 0 {
            match reading(run, stage, source, buffers) {
                Ok(rows) => head_rows = Some(rows),
                Err(fault) => {
                    hard = Some(ChainFault { node: *node, fault });
                    break;
                }
            }
        }
        if let Some(right) = right {
            let earlier = rights.iter().position(
                |taken| matches!(taken, ChainRows::Node(read, _) if right == RowSource::Node(*read)),
            );
            let rows = match (source, earlier) {
                (RowSource::Node(read), _) if right == RowSource::Node(read) => Ok(ChainRows::Head),
                (_, Some(earlier)) => Ok(ChainRows::Right(RightPlace(earlier))),
                _ => reading(run, stage, right, buffers),
            };
            match rows {
                Ok(rows) => {
                    if let Some(scratch) = stages.get_mut(at) {
                        scratch.own.right = RightPlace(rights.len());
                    }
                    rights.push(rows);
                }
                Err(fault) => {
                    hard = Some(ChainFault { node: *node, fault });
                    break;
                }
            }
        }
        let head = head_rows
            .as_ref()
            .ok_or(PortableOperatorFault::WrongOperation)?;
        // When counting is on, a stage's preparation is its own time: a probe's right side indexed, an extension resolved.
        let span = timed(buffers);
        let readied = prepared(
            Prepare {
                plan,
                head,
                rights: &rights,
                nodes,
                stage,
                tail: &mut output,
            },
            run,
            buffers,
            stages,
        );
        timed_to(buffers, span, *node);
        if let Some(fault) = readied? {
            hard = Some(fault);
            break;
        }
    }
    let Some(head_rows) = head_rows else {
        given_back(buffers, ChainRows::Head, rights);
        closed_all(buffers, stages);
        return Err(hard.map_or(PortableOperatorFault::WrongOperation, |first| first.fault));
    };
    let first_probe = nodes
        .iter()
        .position(|node| {
            physical(plan, *node).is_ok_and(|node| matches!(node.lowering, Lowering::Probe { .. }))
        })
        .map(StagePlace);
    let mut push = Push {
        plan,
        run,
        buffers,
        head: &head_rows,
        rights: &rights,
        tail: &mut output,
        hard,
        last: StagePlace(length.saturating_sub(1)),
        first_probe,
    };
    let answered = pushed_all(&mut push, nodes, row, head_chunk, stages);
    let hard = push.hard.take();
    closed_all(buffers, stages);
    let pending = pending_of(&head_rows);
    given_back(buffers, head_rows, rights);
    if let Some(first) = hard {
        return Err(first.fault);
    }
    let fault = answered?;
    hold(plan, buffers, tail, output, fault.or(pending));
    Ok(())
}

/// Closes every shared index a probe of the chain opened.
fn closed_all(buffers: &mut FileBuffers, stages: &mut [StageScratch]) {
    for stage in stages.iter_mut() {
        if let Some(at) = stage.own.shared.take() {
            closed(buffers, at);
        }
    }
}

/// Lends a held node the chain has taken back to the buffers, untouched, while a right side is readied.
fn lent(rows: &mut ChainRows<'_>, buffers: &mut FileBuffers) {
    let ChainRows::Node(read, _) = rows else {
        return;
    };
    let read = *read;
    if let ChainRows::Node(_, taken) = std::mem::replace(rows, ChainRows::Untaken(read)) {
        let at = usize::try_from(read.0).unwrap_or(usize::MAX);
        if let Some(held) = buffers.nodes.get_mut(at) {
            *held = Some(taken.rows);
        }
        if let Some(pending) = buffers.pending.get_mut(at) {
            *pending = taken.pending;
        }
    }
}

/// Takes a node lent back for a right side's readying again.
///
/// # Errors
///
/// `NotRun` for a node the buffers no longer hold.
fn retaken(
    rows: &mut ChainRows<'_>,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    if let ChainRows::Untaken(read) = rows {
        let read = *read;
        *rows = ChainRows::Node(read, take(buffers, read)?);
    }
    Ok(())
}

/// Prepares the stage `prepare.stage` of the chain `prepare.nodes`, as the held plan readies a node before its rows: its output kinds from its input's,
/// through `stage_prepared`. A stage whose preparation faults stops before its first row, and the stages after it are
/// prepared over its empty kinds. A probe of a shared index opens it. Answers the stage's hard fault, the chain's first.
///
/// # Errors
///
/// A stage the chain does not hold.
fn prepared(
    prepare: Prepare<'_, '_, '_>,
    run: &mut KernelRun<'_>,
    buffers: &mut FileBuffers,
    stages: &mut [StageScratch],
) -> Result<Option<ChainFault>, PortableOperatorFault> {
    let Prepare {
        plan,
        head,
        rights,
        nodes,
        stage: chain_stage,
        tail,
    } = prepare;
    let at = nodes
        .iter()
        .position(|node| *node == chain_stage.node)
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let last = stages.len().saturating_sub(1);
    let (before, from) = stages.split_at_mut(at);
    let stage = from
        .first_mut()
        .ok_or(PortableOperatorFault::WrongOperation)?;
    stage.fault = None;
    if let Lowering::Probe {
        index: ProbeIndex::Shared(place),
        ..
    } = chain_stage.lowering
    {
        opened(plan, buffers, *place);
        stage.own.shared = Some(*place);
    }
    let input: &[CellKind] = before
        .last()
        .map_or_else(|| rows_kinds(head), |stage| &stage.out.kinds);
    let right = right_of(chain_stage.lowering).and_then(|_| {
        rights
            .get(stage.own.right.0)
            .map(|rows| resolved_rows(rows, head, rights))
    });
    match stage_prepared(run, buffers, chain_stage, input, &mut stage.own, right) {
        Ok(prepared) => {
            stage.fault = prepared.fault;
            if at == last {
                tail.kinds = prepared.kinds;
            } else {
                stage.out.kinds = prepared.kinds;
            }
            Ok(None)
        }
        Err(fault) => {
            stage.fault = Some(fault.clone());
            Ok(Some(ChainFault {
                node: chain_stage.node,
                fault,
            }))
        }
    }
}
