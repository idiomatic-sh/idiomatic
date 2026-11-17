//! The rows a chain of streamed stages reads besides its stages' own: its nodes, walked back from its tail; its head stage's
//! source and each probe's right side, read for the chain and given back after it; and those rows read in place.
use super::super::super::types::{ChainRows, ChainStage};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::scan_bound::item::held;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::{facts, rows_of};
use crate::domain::physical_plan::collections::kernels::scan_node::item::{give_back, take};
use crate::domain::physical_plan::collections::kernels::scan_roots::item::{rooted, tree_read};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::{Lowering, ProbeNode};
use crate::domain::physical_plan::types::physical_plan::value::{PhysicalNode, PhysicalPlan};
use crate::domain::physical_plan::types::stream_scratch::value::RightPlace;
use crate::domain::physical_plan::types::{
    InputRows, KernelRun, NodeFlow, NodeReaders, NodeRows, PlanNodeId, RowSource, TakenRows,
    TreeRead,
};
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::rows::types::RowCount;

/// The node at `node`.
pub(in crate::domain::physical_plan::service::run::pipeline) fn physical(
    plan: &PhysicalPlan,
    node: PlanNodeId,
) -> Result<&PhysicalNode, PortableOperatorFault> {
    plan.nodes
        .get(usize::try_from(node.0).unwrap_or(usize::MAX))
        .ok_or(PortableOperatorFault::WrongOperation)
}

/// Fills `chain` with the nodes that stream into `tail`, from the head stage to `tail`: each first source that streams and
/// holds no rows in this file.
pub(in crate::domain::physical_plan::service::run::pipeline) fn walked(
    plan: &PhysicalPlan,
    tail: PlanNodeId,
    buffers: &FileBuffers,
    chain: &mut Vec<PlanNodeId>,
) -> Result<(), PortableOperatorFault> {
    chain.clear();
    let mut at = tail;
    loop {
        chain.push(at);
        match physical(plan, at)?.lowering.sources().next() {
            Some(RowSource::Node(read))
                if physical(plan, read)?.flow == NodeFlow::Streamed
                    && buffers.rows(read) == NodeRows::Absent =>
            {
                at = read;
            }
            Some(_) => break,
            None => return Err(PortableOperatorFault::WrongOperation),
        }
    }
    chain.reverse();
    Ok(())
}

/// Reads `source` for `reader`, in place: a tree relation's rows are read from the elements a chunk at a time, so a text_map
/// stage, which mints while it reads, holds no borrow of them.
pub(in crate::domain::physical_plan::service::run::pipeline) fn reading<'r>(
    run: &KernelRun<'r>,
    reader: ChainStage<'_>,
    source: RowSource,
    buffers: &mut FileBuffers,
) -> Result<ChainRows<'r>, PortableOperatorFault> {
    let inputs = run.inputs;
    Ok(match source {
        RowSource::Facts(relation) => ChainRows::Facts(facts(inputs, relation)?),
        RowSource::Tree(relation) => ChainRows::Tree(rooted(inputs, reader.node, relation)),
        RowSource::Node(read) => ChainRows::Node(read, take(buffers, read)?),
        // A withheld input faults where it is first read, never as empty rows.
        RowSource::Frontier(place) => ChainRows::Held(held(inputs.frontier(place))?),
        RowSource::Observation(place) => ChainRows::Held(held(inputs.observation(place))?),
    })
}

/// A probe's right source: its lowering's second.
pub(in crate::domain::physical_plan::service::run::pipeline) fn right_of(
    lowering: &Lowering,
) -> Option<RowSource> {
    match lowering {
        Lowering::Probe(ProbeNode { right, .. }) => Some(*right),
        Lowering::Filter(_)
        | Lowering::Lookup(_)
        | Lowering::Project(_)
        | Lowering::Extend(_)
        | Lowering::Distinct(_)
        | Lowering::Group(_)
        | Lowering::Reach(_)
        | Lowering::SeededReach(_)
        | Lowering::Axis(_)
        | Lowering::OrderedWalk(_)
        | Lowering::Worklist(_)
        | Lowering::Components(_)
        | Lowering::TextMap(_)
        | Lowering::OrderedMatch(_)
        | Lowering::Render(_)
        | Lowering::ScanBound(_) => None,
    }
}

/// Gives back every node the chain read: the head's source and each probe's own right side, each after all its reads.
pub(in crate::domain::physical_plan::service::run::pipeline) fn given_back(
    buffers: &mut FileBuffers,
    head: ChainRows<'_>,
    rights: Vec<ChainRows<'_>>,
) {
    let reads = |count| NodeReaders(1 + u32::try_from(count).unwrap_or(u32::MAX));
    let head_reads = reads(
        rights
            .iter()
            .filter(|rows| matches!(rows, ChainRows::Head))
            .count(),
    );
    let right_reads: Vec<NodeReaders> = (0..rights.len())
        .map(|at| {
            reads(
                rights
                    .iter()
                    .filter(|rows| matches!(rows, ChainRows::Right(RightPlace(earlier)) if *earlier == at))
                    .count(),
            )
        })
        .collect();
    if let ChainRows::Node(read, taken) = head {
        give_back(buffers, read, taken, head_reads);
    }
    for (rows, readings) in rights.into_iter().zip(right_reads) {
        if let ChainRows::Node(read, taken) = rows {
            give_back(buffers, read, taken, readings);
        }
    }
}

/// The rows `rows` stands for: a probe's right side that is the head's rows, or an earlier probe's, followed to them.
pub(in crate::domain::physical_plan::service::run::pipeline) fn resolved_rows<'a, 'r>(
    rows: &'a ChainRows<'r>,
    head: &'a ChainRows<'r>,
    rights: &'a [ChainRows<'r>],
) -> &'a ChainRows<'r> {
    match rows {
        ChainRows::Head => head,
        ChainRows::Right(RightPlace(at)) => rights
            .get(*at)
            .filter(|earlier| !matches!(earlier, ChainRows::Right(_)))
            .map_or(rows, |earlier| resolved_rows(earlier, head, rights)),
        ChainRows::Facts(_)
        | ChainRows::Tree(_)
        | ChainRows::Node(..)
        | ChainRows::Held(_)
        | ChainRows::Untaken(_) => rows,
    }
}

/// The kinds of `rows`; none for rows that stand for others.
pub(in crate::domain::physical_plan::service::run::pipeline) fn rows_kinds<'a>(
    rows: &'a ChainRows<'_>,
) -> &'a [CellKind] {
    match rows {
        ChainRows::Facts(view) => view.kinds,
        ChainRows::Tree(roots) => roots.kinds,
        ChainRows::Node(_, TakenRows { rows: table, .. }) => &table.kinds,
        ChainRows::Held(table) => &table.kinds,
        ChainRows::Head | ChainRows::Right(_) | ChainRows::Untaken(_) => &[],
    }
}

/// How many rows `rows` has; none for rows that stand for others.
pub(in crate::domain::physical_plan::service::run::pipeline) fn rows_count(
    rows: &ChainRows<'_>,
) -> RowCount {
    match rows {
        ChainRows::Facts(view) => rows_of(view),
        ChainRows::Tree(roots) => RowCount(roots.candidates.len()),
        ChainRows::Node(_, TakenRows { rows: table, .. }) => table.count,
        ChainRows::Held(table) => table.count,
        ChainRows::Head | ChainRows::Right(_) | ChainRows::Untaken(_) => RowCount(0),
    }
}

/// The fault `rows` raised after their rows: a held node's.
pub(in crate::domain::physical_plan::service::run::pipeline) fn pending_of(
    rows: &ChainRows<'_>,
) -> Option<PortableOperatorFault> {
    match rows {
        ChainRows::Node(_, taken) => taken.pending.clone(),
        ChainRows::Facts(_)
        | ChainRows::Tree(_)
        | ChainRows::Held(_)
        | ChainRows::Head
        | ChainRows::Right(_)
        | ChainRows::Untaken(_) => None,
    }
}

/// `rows` read in place: a tree relation's from `elements`, its read kept in `tree`.
pub(in crate::domain::physical_plan::service::run::pipeline) fn in_place<'a>(
    rows: &'a ChainRows<'_>,
    elements: &'a Elements,
    tree: &'a mut Option<TreeRead<'a>>,
) -> InputRows<'a> {
    match rows {
        ChainRows::Facts(view) => InputRows::Facts(view),
        ChainRows::Tree(roots) => InputRows::Tree(tree.insert(tree_read(*roots, elements))),
        ChainRows::Node(_, TakenRows { rows: table, .. }) => InputRows::Table(table),
        ChainRows::Held(table) => InputRows::Table(table),
        ChainRows::Head | ChainRows::Right(_) | ChainRows::Untaken(_) => InputRows::Row(&[], &[]),
    }
}
