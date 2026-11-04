//! Application-owned admission and acquisition for the fused column kernel.
//! The domain kernel receives only a prepared index and borrowed semantic inputs.

use crate::domain::column_kernel::{
    service as kernel,
    types::{Chain, KernelCounts},
};
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::table_dedup::service::distinct;
use crate::domain::table_model::{
    reading::Reading,
    service::{column_in, rectangular, reset},
    types::Rows,
};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::{service::kinds as relation_kinds, types::CellKind};
use std::sync::Arc;

pub(super) fn execute_publication(
    semantics: &dyn crate::domain::operator::contract::types::PrimitiveSemantics,
    chain: &Chain,
    read: &dyn Reading<'_>,
    owner: &crate::application::file_dag::relations::RelationOwner,
    publication: &crate::application::file_dag::relations::PublishedInput,
    context: &mut OperatorContext<'_>,
    target: &mut Table,
    kinds: &[CellKind],
    counts: &mut KernelCounts,
) -> Result<bool, OperatingError> {
    let Some(selection) = owner.selected_descriptor(publication.as_ref())? else {
        return Ok(false);
    };
    let Rows::Stored(right_source) = read.rows(chain.right)? else {
        return Ok(false);
    };
    let Rows::Stored(left_source) = read.rows(selection.relation)? else {
        return Ok(false);
    };
    if !rectangular(left_source)
        || !rectangular(right_source)
        || left_source.kinds != relation_kinds(selection.relation)
        || right_source.kinds != relation_kinds(chain.right)
        || selection.columns.len() != selection.kinds.len()
        || selection
            .columns
            .iter()
            .zip(&selection.kinds)
            .any(|(column, kind)| column_in(left_source.kinds, *column) != Some(kind))
        || selection.rows.iter().any(|row| {
            *row as usize
                >= left_source
                    .columns
                    .first()
                    .map_or(0, |column| column.0.len())
        })
    {
        return Ok(false);
    }
    if selection.rows.is_empty() {
        reset(target, kinds);
        counts.eligible_chains += 1;
        return Ok(true);
    }
    if !owner.derived.relation_eligible(selection.relation)
        || !owner.derived.relation_eligible(chain.right)
        || !owner
            .derived
            .can_retain_equality(chain.right, right_source, &chain.right_on)
    {
        return Ok(false);
    }
    let Some(index_plan) = chain.index.as_ref() else {
        return Ok(false);
    };
    let left = owner.selected_rows(publication.as_ref(), left_source)?;
    let right = Rows::Stored(right_source);
    let _consumer = crate::application::file_dag::route_counts::consumer("column");
    let acquired = owner.stored_equality(index_plan, chain.right, right_source)?;
    let ryo_derived::domain::contract::types::DerivedView::Equality(index) = acquired.view()?
    else {
        return Err(OperatingError::WrongOperation);
    };
    let outcome = kernel::execute_with_index(
        semantics,
        chain,
        &left,
        &right,
        right_source,
        index,
        context.symbols.as_ref(),
        ByteSlice(context.source.0),
        target,
        kinds,
    )?;
    let Some((rows, cells, hits)) = outcome else {
        reset(target, kinds);
        return Ok(false);
    };
    distinct(target, &mut context.scratch.join_scratch.seen);
    counts.eligible_chains += 1;
    counts.covered_rows += rows;
    counts.decoded_cells += cells;
    counts.joined_hits += hits;
    if tracing::enabled!(target: "ryo::diagnostic::column_kernel", tracing::Level::TRACE) {
        tracing::trace!(target: "ryo::diagnostic::column_kernel", strategy = "column", rows, decoded_cells = cells, joined_hits = hits, left_relation = ?selection.relation, right_relation = ?chain.right);
    }
    Ok(true)
}

use ryo_values::domain::rows::types::Table;
