//! Application-owned admission and acquisition for the fused column kernel.
//! The domain kernel receives only a prepared index and borrowed semantic inputs.

use crate::application::file_dag::derived::indexes;
use crate::domain::column_kernel::{
    service as kernel,
    types::{Chain, KernelCounts, KernelPlan},
};
use crate::domain::coverage::types::Partial;
use crate::domain::derived_views::{
    collections::views::registration::registered,
    types::{DerivedKind, ViewRequest},
};
use crate::domain::operating::{errors::OperatingError, types::OperatorContext};
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::operator::types::PlanLayout;
use crate::domain::table_dedup::service::distinct;
use crate::domain::table_model::{
    reading::Reading,
    service::{column_in, rectangular, reset},
    types::{Rows, SelectedRows, StoredSelection, Table},
};
use ryo_language::domain::{
    relations::{service::kinds as relation_kinds, types::CellKind},
    symbols::types::ByteSlice,
};
use ryo_values::domain::plans::types::Plan;
use std::sync::Arc;

/// Prepare one immutable equality requirement per admitted physical chain.
/// Invalid optional metadata leaves the ordinary executor in charge.
#[must_use]
pub(crate) fn compile(plan: &Plan, layout: &PlanLayout) -> KernelPlan {
    let mut prepared = kernel::compile(plan, layout);
    for chain in prepared.0.iter_mut().flatten() {
        let schema = [InputSchema {
            kinds: relation_kinds(chain.right),
            tree: None,
        }];
        let request = ViewRequest::Equality {
            columns: chain.right_on.clone(),
        };
        if let Ok(index) = registered(DerivedKind::Equality).prepare(&request, &schema)
            && index.metadata.is_ok()
        {
            chain.index = Some(Arc::new(index));
        }
    }
    prepared
}

/// Preserve the old fast-path admission and fallback order. A refusal leaves
/// the target empty and returns false so the common engine evaluates the step.
pub(crate) fn execute(
    chain: &Chain,
    read: &dyn Reading<'_>,
    selected: &[Option<Arc<StoredSelection>>],
    partial: &[Partial],
    context: &mut OperatorContext<'_>,
    target: &mut Table,
    kinds: &[CellKind],
    counts: &mut KernelCounts,
) -> Result<bool, OperatingError> {
    if !partial.is_empty() {
        return Ok(false);
    }
    let Some(selection) = selected.get(chain.left_step).and_then(Option::as_ref) else {
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
    let Some(cache) = context.stored_indexes.as_deref_mut() else {
        return Ok(false);
    };
    if !cache.eligible.contains(&selection.relation)
        || !cache.eligible.contains(&chain.right)
        || !indexes::can_retain(cache, chain.right, right_source, &chain.right_on)
    {
        return Ok(false);
    }
    let Some(index_plan) = chain.index.as_ref() else {
        return Ok(false);
    };
    let left = Rows::Selected(SelectedRows {
        source: left_source,
        selection: Arc::clone(selection),
    });
    let right = Rows::Stored(right_source);
    let mut local = std::mem::take(&mut context.scratch.join_scratch.order);
    let outcome = indexes::acquire(
        index_plan,
        cache,
        chain.right,
        right_source,
        &chain.right_on,
        &mut local,
    )
    .ok()
    .and_then(|acquired| {
        let index = acquired.shared.as_deref().unwrap_or(&local);
        kernel::execute_with_index(
            chain,
            &left,
            &right,
            right_source,
            index,
            context.symbols.as_ref(),
            ByteSlice(context.source.0),
            target,
            kinds,
        )
    });
    context.scratch.join_scratch.order = local;
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
