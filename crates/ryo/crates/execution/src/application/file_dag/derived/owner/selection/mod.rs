//! Application policy for prepared compact selections and their shared file backend.
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use crate::domain::{
    derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch},
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::views::BorrowedRows,
    stored_selection::types::Selector,
    table_model::{
        service::check_columns,
        types::{SelectedRows, StoredSelection},
    },
};
use ryo_relations::domain::relations::types::ColumnsView;
use std::sync::Arc;
pub(super) mod candidates;
mod fanout;
mod literals;
pub(in crate::application::file_dag) mod preparation;
use crate::application::file_dag::derived::owner::DerivedOwner;
use crate::application::file_dag::derived::owner::acquisition::selection::{
    predicate::service::matches,
    service::{SelectionSource, histogram, shared_positions},
    types::RowBound,
};
use crate::application::file_dag::relations::{RelationOwner, admission::service::BorrowedInput};
pub(in crate::application::file_dag::derived::owner) mod types;
use candidates::service::candidates;
pub(in crate::application::file_dag::derived::owner) struct SelectionOutput<'a> {
    pub(in crate::application::file_dag::derived::owner) rows: SelectedRows<'a>,
    pub(in crate::application::file_dag::derived::owner) resources: [Option<
        crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
    >; 2],
}
/// Produce a compact handle only after the provider has been read successfully by the caller.
pub(in crate::application::file_dag::derived::owner) fn select<'a>(
    cache: &mut CacheBackend,
    resources: crate::application::file_dag::derived::owner::acquisition::selection::service::SelectionSource<'_>,
    program: &Arc<Selector>,
    source: ColumnsView<'a>,
    context: &mut OperatorContext<'_>,
) -> Result<Option<SelectionOutput<'a>>, OperatingError> {
    if cache.eligible.contains(&program.relation) {
        make(cache, resources, program, source, context)
    } else {
        Ok(None)
    }
}

/// Validate projected columns before cache acquisition and leave matching ownership independent of projection.
fn make<'a>(
    cache: &mut CacheBackend,
    resources: crate::application::file_dag::derived::owner::acquisition::selection::service::SelectionSource<'_>,
    program: &Arc<Selector>,
    source: ColumnsView<'a>,
    context: &mut OperatorContext<'_>,
) -> Result<Option<SelectionOutput<'a>>, OperatingError> {
    check_columns(source.kinds, &program.columns)?;
    if let Some(entry) = histogram(cache, program) {
        entry.calls.0 += 1;
    }
    let population = source.columns.first().map_or(0, |column| column.0.len());
    let Some(views) = cache
        .selection_views
        .as_ref()
        .and_then(|prepared| prepared.by_selector.get(&(Arc::as_ptr(program) as usize)))
        .cloned()
    else {
        return Ok(None);
    };
    #[cfg(test)]
    {
        cache.selector_descriptor = Some(
            crate::application::file_dag::derived::owner::cache::types::DescriptorAddress(
                Arc::as_ptr(&views) as usize,
            ),
        );
    }
    if views
        .predicate
        .schemas
        .first()
        .is_none_or(|kinds| kinds.as_slice() != source.kinds)
    {
        return Ok(None);
    }
    let predicate = matches(
        cache, resources, program, &views, source, context, candidates,
    )?;
    let rows = shared_positions(&predicate, RowBound(population), RowBound(population))?.0;
    let mut dependencies = [Some(predicate), None];
    // The shared result above always contains predicate matches, never distinct representatives.
    // Each terminal projection owns its representative IDs independently of other output maps.
    let rows = if program.distinct == ryo_values::domain::plan_parts::types::RepeatedRows::Dropped {
        let acquired = resources.acquire(
            crate::application::file_dag::route_counts::types::BuildRoute::SelectorDistinct,
            cache,
            &views.distinct,
            crate::application::file_dag::derived::owner::ViewContext::for_predicate(
                &views.predicate,
            ),
            DerivedInputs::Distinct {
                rows: BorrowedRows::Stored(source),
                positions: &rows,
            },
            &mut DerivedScratch::default(),
            None,
        )?;
        let result = shared_positions(&acquired, RowBound(population), RowBound(rows.len()))?.0;
        dependencies[1] = Some(acquired);
        result
    } else {
        rows
    };
    let kinds = program
        .columns
        .iter()
        .map(|column| source.kinds[column.0 as usize])
        .collect();
    Ok(Some(SelectionOutput {
        resources: dependencies,
        rows: SelectedRows {
            source,
            selection: Arc::new(StoredSelection {
                relation: program.relation,
                rows: crate::domain::stored_selection::types::SelectedPositions(rows),
                columns: program.columns.clone(),
                kinds,
                identity: (program.identity).clone(),
            }),
        },
    }))
}

#[cfg(test)]
mod retention;

#[cfg(test)]
pub(crate) mod fixtures;

#[cfg(test)]
mod policy;

impl DerivedOwner {
    pub(in crate::application::file_dag) fn selector_plan(
        &self,
        program: &Arc<Selector>,
    ) -> Result<Option<Arc<DerivedContract>>, OperatingError> {
        let backend = self.backend.borrow();
        let backend = backend.as_ref().ok_or(OperatingError::NotRun)?;
        let plans = backend
            .selection_views
            .as_ref()
            .ok_or(OperatingError::NotRun)?;
        let views = plans
            .by_selector
            .get(&(Arc::as_ptr(program) as usize))
            .ok_or(OperatingError::NotRun)?;
        if backend.selector_repetition_failed
            == crate::application::file_dag::derived::owner::cache::types::RepetitionTracking::Failed
        {
            return Ok(None);
        }
        if views.group.0 >= plans.groups.0 {
            return Err(OperatingError::NotRun);
        }
        Ok(Some(Arc::clone(&views.predicate)))
    }
    pub(in crate::application::file_dag) fn select<'f>(
        &self,
        program: &Arc<Selector>,
        source: ColumnsView<'f>,
        relation_owner: &RelationOwner,
        publication: &crate::application::file_dag::relations::handle::service::PublishedInput,
        proof: &BorrowedInput<'_>,
        context: &mut OperatorContext<'_>,
    ) -> Result<
        Option<crate::application::file_dag::relations::handle::service::PublishedInput>,
        OperatingError,
    > {
        let resources = SelectionSource::admit(self, relation_owner, publication, proof, source)?;
        let produced = {
            let mut backend = self.backend.borrow_mut();
            let Some(backend) = backend.as_mut() else {
                return Ok(None);
            };
            select(backend, resources, program, source, context)?
        };
        let Some(produced) = produced else {
            return Ok(None);
        };
        crate::application::file_dag::relations::selection::publish(
            relation_owner,
            publication,
            proof,
            produced.rows.selection,
            produced.resources,
        )
        .map(Some)
    }
}
