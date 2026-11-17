//! The `lookup` item of the `kernels` collection: the rows of a shared `hash_index` whose key is a constant text, in source row
//! order, each then kept by all its select's comparisons: what the plain `filter` keeps.

use super::super::filter::item::{filter, keep, kept_kinds, readied};
use super::super::hash_index::item::{find_text, next_of, texts};
#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{decode, facts, rows_of};
use super::super::scan_node::item::hold;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::growth::types::{Growth, Operator};
use crate::domain::physical_plan::collections::kernels::scan_node::item::fed_with;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::keyed_rows::value::{IndexKeyHash, NO_ROW};
use crate::domain::physical_plan::types::lowering::value::{LookupNode, Lowering};
use crate::domain::physical_plan::types::physical_plan::value::{PhysicalNode, PhysicalPlan};
use crate::domain::physical_plan::service::indexes::service::opened;
use crate::domain::physical_plan::types::{
    IndexKey, IndexPlace, KernelRun, NodeFed, NodeReaders, PlanNodeId, RowSource,
};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `lookup`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct LookupItem;

#[cfg(test)]
impl KernelsItem for LookupItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = { matches!(lowering, Lowering::Lookup(_)) };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Answers lookup `node` of the shared index `index`: the rows whose key is its constant, in source row order, kept by all its
/// comparisons. The index is built from one read of its relation by the file's first lookup of it and kept for the file's
/// other lookups of it, so a file answers only the lookups it runs, not every lookup of the program.
///
/// # Errors
///
/// A refused relation; `NoSuchColumn` for a kept column past the relation's rows; `WrongOperation` for a node that is no
/// lookup of a text index of a relation.
pub(crate) fn lookups(
    plan: &PhysicalPlan,
    run: &KernelRun<'_>,
    node: PlanNodeId,
    index: IndexPlace,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    let shared = plan
        .indexes
        .get(index.0)
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let (RowSource::Facts(relation), IndexKey::Text(column)) = (shared.source, &shared.key) else {
        return Err(PortableOperatorFault::WrongOperation);
    };
    let place = usize::try_from(node.0).unwrap_or(usize::MAX);
    let Some(PhysicalNode {
        lowering: Lowering::Lookup(LookupNode { key, select, .. }),
        ..
    }) = plan.nodes.get(place)
    else {
        return Err(PortableOperatorFault::WrongOperation);
    };
    let view = facts(run.inputs, relation)?;
    let texts_of = run.resources.texts();
    // A file's first lookup of the index builds it; its last reader's release, or the file's end, empties it.
    let built = buffers
        .shared
        .indexes
        .get(index.0)
        .is_some_and(|held| held.remaining != NodeReaders(0));
    if !built {
        if let Some(held) = buffers.shared.indexes.get_mut(index.0) {
            texts(&view, *column, &texts_of, &mut held.rows);
        }
        opened(plan, buffers, index);
        #[cfg(test)]
        {
            buffers.builds.0 += 1;
        }
    }
    #[cfg(test)]
    {
        buffers.shared.answered.0 += 1;
    }
    fed_with(
        buffers,
        node,
        NodeFed([rows_of(&view), RowCount(0), RowCount(0)]),
    );
    let mut output = Table {
        kinds: kept_kinds(view.kinds, &select.kept)?,
        ..Table::default()
    };
    let kernel = filter(select, view.kinds, texts_of);
    let FileBuffers {
        shared: held,
        decisions,
        row: decoded,
        written,
        ..
    } = &mut *buffers;
    let (Some(rows), Some(decisions)) = (
        held.indexes.get(index.0).map(|held| &held.rows),
        decisions.get_mut(place),
    ) else {
        return Err(PortableOperatorFault::WrongOperation);
    };
    readied(decisions);
    let mut fault = None;
    let mut row = find_text(rows, IndexKeyHash(key.0)).map_or(NO_ROW, |run| run.first);
    while row != NO_ROW {
        let at = usize::try_from(row.0).unwrap_or(usize::MAX);
        decode(&view, RowIndex(at), decoded);
        if let Err(error) = keep(kernel, run.matcher, decoded, decisions, written, &mut output) {
            fault = Some(error);
            break;
        }
        row = next_of(rows, row);
    }
    hold(plan, buffers, node, output, fault)
}

impl Operator for LookupNode {
    fn growth(&self) -> Growth {
        Growth::AtMostInput
    }
}
