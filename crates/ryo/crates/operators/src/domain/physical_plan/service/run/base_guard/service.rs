//! Spike (ryo-fmx3): a guard that is a select or projection straight over a relation or the tree is answered by reading the
//! relation in place until its first kept row: a pure projection holds a row exactly when its relation does, a select when
//! one row passes its comparisons. The leaf itself is not made, so a guard costs no copy of the file's base rows.
use crate::domain::physical_plan::collections::kernels::filter::item::{filter, holds, readied};
use crate::domain::physical_plan::collections::kernels::scan_facts::item::{decode, facts, rows_of};
use crate::domain::physical_plan::collections::kernels::scan_roots::item::{roots, tree_count, tree_row};
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{KernelRun, PlanNodeId, RowSource};
use ryo_values::domain::rows::types::RowIndex;

/// What reading guard `node`'s base rows in place tells: `Some(true)` when the leaf would hold a row or raise a fault (either
/// way it guards nothing), `Some(false)` when it holds no row and raises none, `None` when the leaf is no base leaf this
/// reads and must be made.
pub(crate) fn base_rows(
    plan: &PhysicalPlan,
    run: &KernelRun<'_>,
    node: PlanNodeId,
    buffers: &mut FileBuffers,
) -> Option<bool> {
    if buffers.counters.is_some() {
        return None;
    }
    let place = usize::try_from(node.0).ok()?;
    let (source, select, kept) = match &plan.nodes.get(place)?.lowering {
        Lowering::Filter { source, select } => (*source, Some(select), &select.kept[..]),
        Lowering::Project { source, .. } | Lowering::Extend { source, .. } => (*source, None, &[][..]),
        _ => return None,
    };
    let select = select.filter(|select| !select.comparisons.is_empty());
    if select.is_some_and(|select| !select.bindings.is_empty()) {
        return None;
    }
    match source {
        RowSource::Facts(relation) => {
            let Ok(view) = facts(run.inputs, relation) else {
                return Some(true);
            };
            let count = rows_of(&view).0;
            if count == 0 || !kept_in(kept, view.kinds.len()) {
                return None;
            }
            let Some(select) = select else {
                return Some(true);
            };
            if never_holds(select, view.kinds) {
                return Some(false);
            }
            // Spike (ryo-fmx3): a partitioned sibling reads only its bucket and the shared rest.
            let listed = crate::spike::fanout()
                && match crate::domain::physical_plan::collections::kernels::partition::item::sibling_rows(plan, run, node, source, buffers, true) {
                    Some(true) => return Some(false),
                    Some(false) => true,
                    None => false,
                };
            let kernel = filter(select, view.kinds, run.resources.texts());
            let decisions = buffers.decisions.get_mut(place)?;
            readied(decisions);
            let row = &mut buffers.row;
            let list: &[u32] = if listed { &buffers.partitions.rows } else { &[] };
            for read in 0..if listed { list.len() } else { count } {
                let at = if listed { list.get(read).map_or(usize::MAX, |at| *at as usize) } else { read };
                decode(&view, RowIndex(at), row);
                match holds(kernel, run.matcher, row, decisions) {
                    Ok(PredicateOutcome::Fails) => {}
                    Ok(_) | Err(_) => return Some(true),
                }
            }
            Some(false)
        }
        RowSource::Tree(relation) => {
            let tree = roots(run.inputs, run.resources.tree(), node, relation);
            let count = tree_count(&tree).0;
            if count == 0 || !kept_in(kept, tree.kinds.len()) {
                return None;
            }
            let Some(select) = select else {
                return Some(true);
            };
            if never_holds(select, tree.kinds) {
                return Some(false);
            }
            // Spike (ryo-fmx3): a partitioned sibling reads only its bucket and the shared rest.
            let listed = crate::spike::fanout()
                && match crate::domain::physical_plan::collections::kernels::partition::item::sibling_rows(plan, run, node, source, buffers, true) {
                    Some(true) => return Some(false),
                    Some(false) => true,
                    None => false,
                };
            let kernel = filter(select, tree.kinds, run.resources.texts());
            let decisions = buffers.decisions.get_mut(place)?;
            readied(decisions);
            let row = &mut buffers.row;
            let list: &[u32] = if listed { &buffers.partitions.rows } else { &[] };
            for read in 0..if listed { list.len() } else { count } {
                let at = if listed { list.get(read).map_or(usize::MAX, |at| *at as usize) } else { read };
                tree_row(&tree, RowIndex(at), row);
                match holds(kernel, run.matcher, row, decisions) {
                    Ok(PredicateOutcome::Fails) => {}
                    Ok(_) | Err(_) => return Some(true),
                }
            }
            Some(false)
        }
        // Spike (ryo-fmx3): a partitioned sibling over a node the open file holds reads only its bucket and the rest, so a
        // killer that is one is answered without making it (a made killer that streams would be held for its reader). Over a
        // node not made yet it is not made only to be asked: it answers as guarding nothing, and the reader runs as it would.
        RowSource::Node(read) => {
            if !crate::spike::fanout() || plan.partitions.of(node).is_none() {
                return None;
            }
            let at = usize::try_from(read.0).ok()?;
            let Some(Some(held)) = buffers.nodes.get(at) else {
                return Some(true);
            };
            let width = held.kinds.len();
            let select = select?;
            if !kept_in(kept, width) {
                return None;
            }
            if crate::domain::physical_plan::collections::kernels::partition::item::sibling_rows(plan, run, node, source, buffers, true)? {
                return Some(false);
            }
            let FileBuffers { nodes, decisions, partitions, .. } = buffers;
            let table = nodes.get(at)?.as_ref()?;
            let kernel = filter(select, &table.kinds, run.resources.texts());
            let decisions = decisions.get_mut(place)?;
            readied(decisions);
            for listed in &partitions.rows {
                let row = crate::domain::physical_plan::collections::kernels::scan_facts::item::table_row(table, RowIndex(*listed as usize));
                match holds(kernel, run.matcher, row, decisions) {
                    Ok(PredicateOutcome::Fails) => {}
                    Ok(_) | Err(_) => return Some(true),
                }
            }
            Some(false)
        }
        RowSource::Frontier(_) | RowSource::Observation(_) => None,
    }
}

/// Whether every kept column is one of the row's `width` columns: a select keeping another faults even with no row kept.
fn kept_in(kept: &[ryo_values::domain::plan_parts::types::ColumnIndex], width: usize) -> bool {
    kept.iter().all(|column| usize::try_from(column.0).is_ok_and(|at| at < width))
}

/// Whether the select's first comparison holds for no row and faults on none: a node or number column compared with itself
/// by `!=`, `<` or `>`.
fn never_holds(
    select: &crate::domain::select::types::SelectParts,
    kinds: &[ryo_relations::domain::relations::types::CellKind],
) -> bool {
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::plan_parts::types::{Operand, Test};
    let Some(first) = select.comparisons.first() else {
        return false;
    };
    let (Operand::Column(left), Operand::Column(right)) = (&first.left, &first.right) else {
        return false;
    };
    left == right
        && matches!(first.test, Test::NotEqual | Test::LessThan | Test::GreaterThan)
        && usize::try_from(left.0)
            .ok()
            .and_then(|at| kinds.get(at))
            .is_some_and(|kind| matches!(kind, CellKind::Node | CellKind::Number))
}
