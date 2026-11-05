//! The live tree-walk shape helpers used by prepared traversal and schema validation.

use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::rows::errors::TableModelError;
use {ryo_values::domain::rows::service::column_in, crate::domain::table_model::service::reset};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};
use ryo_values::domain::rows::types::Table;

/// The columns of the rows a walk from `from` makes.
///
/// # Errors
///
/// An output column that the side it names does not have.
pub(crate) fn walk_kinds(
    from: &[CellKind],
    reached: &[CellKind],
    output: &[JoinColumn],
    out: &mut Table,
) -> Result<(), OperatingError> {
    reset(out, &[]);
    if output.is_empty() {
        out.kinds.extend_from_slice(from);
        out.kinds.push(CellKind::Node);
        return Ok(());
    }
    for column in output {
        let kinds = match column.side {
            Side::Left => from,
            Side::Right => reached,
        };
        out.kinds.push(
            *column_in(kinds, column.column).ok_or(TableModelError::NoSuchColumn {
                index: column.column.0,
            })?,
        );
    }
    Ok(())
}

/// The column of the domain that holds the entity a walk reaches: the domain's id, through its view (a column the view does not name is the domain's own).
pub(crate) fn entity_of(domain_view: &[ColumnIndex], domain_id: ColumnIndex) -> ColumnIndex {
    match domain_view.get(usize::try_from(domain_id.0).unwrap_or(usize::MAX)) {
        Some(moved) => *moved,
        None => domain_id,
    }
}
