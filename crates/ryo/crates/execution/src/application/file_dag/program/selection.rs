//! A compact selected publication can be minted only by its admitted producer.
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::runtime::service::KernelPermit;
use crate::application::file_dag::relations::{RelationOwner, handle::service::PublishedInput};
use crate::domain::{
    executable_program::types::SelectionAlternative,
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::{types::WorkCounters, views::BorrowedRows},
    table_model::{service::rectangular, types::Rows},
};
use std::rc::Rc;

/// No provider callbacks or dependency requests enter this stage. The source is
/// the borrowed representation of the exact input already admitted by runtime.
pub(super) fn acquire(
    prepared: &PreparedProgram,
    permit: &KernelPermit<'_>,
    shape: &SelectionAlternative,
    rows: &Rows<'_>,
    owner: &Rc<RelationOwner>,
    context: &mut OperatorContext<'_>,
    work: &WorkCounters,
    walk: &super::native_walk::WalkPartition<'_>,
    kernels: &dyn super::super::executor::PrimitiveExecutor,
) -> Result<Option<PublishedInput>, OperatingError> {
    let input = permit.selection_input(shape)?;
    let publication = input.publication(owner)?;
    let proof = owner.admit(publication.as_ref(), BorrowedRows::from(rows))?;
    if owner.stable() == crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Ineligible {
        return Ok(None);
    }
    let Rows::Stored(columns) = rows else {
        return Ok(None);
    };
    if !rectangular(*columns)
        || columns
            .columns
            .first()
            .is_some_and(|column| column.0.len() > u32::MAX as usize)
    {
        return Ok(None);
    }
    // An empty primary has no representatives to acquire: the ordinary route
    // answers it without building any derived payload.
    if shape.selector().distinct
        && columns
            .columns
            .first()
            .is_none_or(|column| column.0.is_empty())
    {
        return Ok(None);
    }
    if proof.physical_shape()
        != crate::domain::table_model::collections::representations::types::PhysicalShape::Complete
    {
        return Ok(None);
    }
    if let Some(shared) = walk.acquire(
        prepared,
        permit,
        rows,
        super::native_walk::WalkDelivery::Stored,
        context,
        kernels,
    )? {
        let selector = shape.selector();
        let kinds = selector
            .columns
            .iter()
            .map(|column| {
                columns
                    .kinds
                    .get(column.0 as usize)
                    .copied()
                    .ok_or(OperatingError::ShapeMismatch)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if kinds != permit.output_kinds()? {
            return Err(OperatingError::ShapeMismatch);
        }
        let selection = {
            let result = shared.result();
            work.found(0, shared.visits().0);
            result.completion.clone()?;
            std::sync::Arc::new(crate::domain::table_model::types::StoredSelection {
                relation: selector.relation,
                rows: std::sync::Arc::new(result.positions.0.clone()),
                columns: selector.columns.clone(),
                kinds,
                identity: std::sync::Arc::clone(&selector.identity),
            })
        };
        // The cached walk borrow ends before owner-issued publication/leases.
        return crate::application::file_dag::relations::selection::publish(
            owner,
            publication,
            &proof,
            selection,
            [None, None],
        )
        .map(Some);
    }
    let Some(plan) = owner.derived.selector_plan(shape.selector())? else {
        return Ok(None);
    };
    if plan
        .schemas
        .first()
        .is_none_or(|schema| schema.as_slice() != columns.kinds)
    {
        return Ok(None);
    }
    let mut rebound = OperatorContext {
        phase: context.phase,
        semantic: context.semantic,
        prepared_matchers: context.prepared_matchers,
        prepared_kinds: context.prepared_kinds,
        bindings: prepared.bindings(shape.producer())?,
        symbols: ryo_language::domain::symbols::service::query_reborrow(&mut context.symbols),
        source: context.source,
        paths: context.paths,
        arenas: context.arenas,
        memo: &mut *context.memo,
        scratch: &mut *context.scratch,
    };
    let before = owner.derived.selector_work()?;
    let result = owner.derived.select(
        shape.selector(),
        *columns,
        owner,
        publication,
        &proof,
        &mut rebound,
    );
    let after = owner.derived.selector_work()?;
    work.found(0, after.0[0].saturating_sub(before.0[0]));
    result
}
