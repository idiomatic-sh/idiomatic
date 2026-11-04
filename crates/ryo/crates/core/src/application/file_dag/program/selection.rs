//! A compact selected publication can be minted only by its admitted producer.
use super::{admission::KernelPermit, facade::PreparedProgram};
use crate::application::file_dag::{
    derived::view::AcquiredView,
    relations::{PublishedInput, RelationOwner},
};
use crate::domain::{
    executable_program::selection::SelectionAlternative,
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::{types::WorkCounters, views::BorrowedRows},
    table_model::{
        service::rectangular,
        types::{Rows, StoredSelection},
    },
};
use std::{rc::Rc, sync::Arc};

pub(in crate::application::file_dag) struct ProducedSelection {
    parts: SelectionParts,
}
pub(in crate::application::file_dag) struct SelectionParts {
    pub source: PublishedInput,
    pub selection: Arc<StoredSelection>,
    pub resources: [Option<AcquiredView>; 2],
}
impl ProducedSelection {
    pub(in crate::application::file_dag) fn into_parts(self) -> SelectionParts {
        self.parts
    }
}
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
) -> Result<Option<ProducedSelection>, OperatingError> {
    let input = permit.selection_input(shape)?;
    let publication = input.publication(owner)?;
    let proof = owner.admit(publication.as_ref(), BorrowedRows::from(rows))?;
    if !owner.stable() {
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
    let (population, _) = proof.identity(owner)?;
    let mut rebound = OperatorContext {
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
    let result = owner
        .derived
        .select(shape.selector(), *columns, population, &mut rebound);
    let after = owner.derived.selector_work()?;
    work.found(0, after.0[0].saturating_sub(before.0[0]));
    let Some(selected) = result? else {
        return Ok(None);
    };
    Ok(Some(ProducedSelection {
        parts: SelectionParts {
            source: publication.clone(),
            selection: selected.rows.selection,
            resources: selected.resources,
        },
    }))
}
