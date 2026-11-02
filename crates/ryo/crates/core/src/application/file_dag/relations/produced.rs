//! The only constructor of a trusted selected publication runs the real producer.
use super::{RelationOwner, RelationResult};
use crate::domain::{
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::views::BorrowedRows,
    stored_selection::types::Selector,
    table_model::{
        collections::representations::types::PhysicalShape,
        types::{Rows, StoredSelection},
    },
};
use ryo_language::domain::relations::{service::kinds, types::Relation};
use std::sync::Arc;

/// Fields are visible only to the producer and its private owner, not primitive items.
pub(crate) struct ProducedSelection {
    pub(super) source: Arc<RelationResult>,
    pub(super) selection: Arc<StoredSelection>,
}
pub(super) fn select<'f>(
    owner: &RelationOwner,
    program: &Arc<Selector>,
    read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    context: &mut OperatorContext<'_>,
) -> Result<Option<ProducedSelection>, OperatingError> {
    if !owner.stable() {
        return Ok(None);
    }
    if context.stored_indexes.as_ref().is_none_or(|cache| {
        !cache
            .selectors
            .iter()
            .any(|candidate| candidate.relation == program.relation)
    }) {
        return Ok(None);
    }
    let raw = read(program.relation)?;
    let Rows::Stored(columns) = &raw else {
        return Ok(None);
    };
    if columns.kinds != kinds(program.relation) {
        return Ok(None);
    }
    let source = owner.source(program.relation, &raw)?;
    if owner.inspect(source.as_ref(), BorrowedRows::from(&raw))? != PhysicalShape::Complete {
        return Ok(None);
    }
    // This producer derives positions from the checked population and derives
    // output kinds from the exact physical projection. It cannot accept caller
    // supplied positions or a caller supplied trusted bit.
    let Some(selected) =
        crate::domain::stored_selection::service::select(program, *columns, context)?
    else {
        return Ok(None);
    };
    Ok(Some(ProducedSelection {
        source,
        selection: selected.selection,
    }))
}
