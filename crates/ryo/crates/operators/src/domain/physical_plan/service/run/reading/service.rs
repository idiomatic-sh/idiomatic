//! What a node's kernels read: each input's rows, lent in place, read from the tree or taken from an earlier node, and given
//! back after.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::facts;
use crate::domain::physical_plan::collections::kernels::scan_node::item::{give_back, take};
use crate::domain::physical_plan::collections::kernels::scan_roots::item::roots;
use crate::domain::physical_plan::service::run::types::Reading;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    InputRows, KernelRun, NodeReaders, PlanNodeId, RowSource,
};

/// What `source` reads, taken for its reader `node`: a relation's rows lent in place, a tree relation's rows at the tree nodes
/// the engine names for the reader, or a ready node's rows.
pub(crate) fn reading<'s>(
    run: &'s KernelRun<'_>,
    node: PlanNodeId,
    source: RowSource,
    buffers: &mut FileBuffers,
) -> Result<Reading<'s>, PortableOperatorFault> {
    match source {
        RowSource::Facts(relation) => facts(run.inputs, relation).map(Reading::Facts),
        RowSource::Tree(relation) => Ok(Reading::Tree(roots(
            run.inputs,
            run.resources.tree(),
            node,
            relation,
        ))),
        RowSource::Node(read) => take(buffers, read).map(|taken| Reading::Node(read, taken)),
    }
}

/// The rows of a reading, read in place.
pub(crate) fn rows_in<'r>(reading: &'r Reading<'_>) -> InputRows<'r> {
    match reading {
        Reading::Facts(view) => InputRows::Facts(view),
        Reading::Tree(tree) => InputRows::Tree(tree),
        Reading::Copied(rows) => InputRows::Table(rows),
        Reading::Node(_, taken) => InputRows::Table(&taken.rows),
    }
}

/// Gives a reading back after `readings` of its node's readers.
pub(crate) fn returned(
    plan: &PhysicalPlan,
    buffers: &mut FileBuffers,
    reading: Reading<'_>,
    readings: NodeReaders,
) {
    if let Reading::Node(read, taken) = reading {
        give_back(plan, buffers, read, taken, readings);
    }
}

/// The fault a read node raised after its rows.
pub(crate) fn pending_of(reading: &Reading<'_>) -> Option<PortableOperatorFault> {
    match reading {
        Reading::Facts(_) | Reading::Tree(_) | Reading::Copied(_) => None,
        Reading::Node(_, taken) => taken.pending.clone(),
    }
}
