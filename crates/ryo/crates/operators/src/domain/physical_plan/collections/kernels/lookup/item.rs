//! The `lookup` item of the `kernels` collection: the rows of a shared `hash_index` whose key is a constant text, in source row
//! order, each then kept by all its select's comparisons: what the plain `filter` keeps.

use super::super::filter::item::{filter, keep, kept_kinds};
use super::super::hash_index::item::{find_text, next_of, texts};
#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{decode, facts};
use super::super::scan_node::item::hold;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::file_buffers::value::{FileBuffers, IndexKeyHash, NO_ROW};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{IndexPlace, KernelRun, Lowering, PhysicalNode};
use ryo_values::domain::rows::types::{RowIndex, Table};

/// `lookup`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct LookupItem;

#[cfg(test)]
impl KernelsItem for LookupItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = { matches!(lowering, Lowering::Lookup { .. }) };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Builds the shared index `index` from one read of its relation and answers every lookup of it: each the rows whose key is
/// its constant, in source row order, kept by all its comparisons.
///
/// # Errors
///
/// A refused relation; `NoSuchColumn` for a kept column past the relation's rows.
pub(crate) fn lookups(
    plan: &PhysicalPlan,
    run: &KernelRun<'_>,
    index: IndexPlace,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    let shared = plan
        .indexes
        .get(index.0)
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let view = facts(run.inputs, shared.relation)?;
    let texts_of = run.resources.texts();
    texts(&view, shared.column, &texts_of, &mut buffers.index);
    for lookup in &shared.lookups {
        let place = usize::try_from(lookup.0).unwrap_or(usize::MAX);
        let (
            Some(PhysicalNode {
                lowering: Lowering::Lookup { key, select, .. },
                ..
            }),
            Some(decisions),
        ) = (plan.nodes.get(place), buffers.decisions.get_mut(place))
        else {
            continue;
        };
        let mut output = Table {
            kinds: kept_kinds(view.kinds, &select.kept)?,
            ..Table::default()
        };
        let kernel = filter(select, view.kinds, texts_of, decisions);
        let mut fault = None;
        let mut row =
            find_text(&buffers.index, IndexKeyHash(key.0)).map_or(NO_ROW, |run| run.first);
        while row != NO_ROW {
            let at = usize::try_from(row.0).unwrap_or(usize::MAX);
            decode(&view, RowIndex(at), &mut buffers.row);
            if let Err(error) = keep(
                kernel,
                run.matcher,
                &buffers.row,
                decisions,
                &mut buffers.written,
                &mut output,
            ) {
                fault = Some(error);
                break;
            }
            row = next_of(&buffers.index, row);
        }
        hold(plan, buffers, *lookup, output, fault);
    }
    Ok(())
}
