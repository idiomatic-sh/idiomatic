//! The only constructor of a trusted selected publication runs the real producer.
use super::{PublishedInput, RelationOwner};
use crate::domain::{
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::views::BorrowedRows,
    stored_selection::types::Selector,
    table_model::{
        collections::representations::types::PhysicalShape,
        types::{Rows, StoredSelection},
    },
};
use ryo_relations::domain::relations::{service::kinds, types::Relation};
use std::sync::Arc;

/// Fields are visible only to the producer and its private owner, not primitive items.
pub(crate) struct ProducedSelection<'f> {
    pub(super) rows: ryo_relations::domain::relations::types::ColumnsView<'f>,
    pub(super) source: PublishedInput,
    pub(super) selection: Arc<StoredSelection>,
    pub(super) resources: [Option<crate::application::file_dag::derived::view::AcquiredView>; 2],
}
pub(super) fn select<'f>(
    owner: &RelationOwner,
    program: &Arc<Selector>,
    read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    context: &mut OperatorContext<'_>,
) -> Result<Option<ProducedSelection<'f>>, OperatingError> {
    if !owner.stable() {
        return Ok(None);
    }
    let Some(prepared) = owner.derived.selector_plan(program)? else {
        return Ok(None);
    };
    let raw = read(program.relation)?;
    let Rows::Stored(columns) = &raw else {
        return Ok(None);
    };
    // Compact selection is optional for unreadable physical layouts; leave
    // their established readable-prefix normalization to ordinary execution.
    if !crate::domain::table_model::service::rectangular(*columns)
        || columns
            .columns
            .first()
            .is_some_and(|column| column.0.len() > u32::MAX as usize)
    {
        return Ok(None);
    }
    if prepared
        .schemas
        .first()
        .is_none_or(|schema| schema.as_slice() != columns.kinds)
    {
        return Ok(None);
    }
    let source = owner.source(program.relation, &raw)?;
    if owner.inspect(source.as_ref(), BorrowedRows::from(&raw))? != PhysicalShape::Complete {
        return Ok(None);
    }
    // This producer derives positions from the checked population and derives
    // output kinds from the exact physical projection. It cannot accept caller
    // supplied positions or a caller supplied trusted bit.
    let proof = owner.admit(source.as_ref(), BorrowedRows::Stored(*columns))?;
    let (population, _) = proof.identity(owner)?;
    let Some(selected) = owner
        .derived
        .select(program, *columns, population, context)?
    else {
        return Ok(None);
    };
    Ok(Some(ProducedSelection {
        rows: *columns,
        source,
        selection: selected.rows.selection,
        resources: selected.resources,
    }))
}
