//! Column projection metadata shared by preparation and empty continuation publication.
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    plans::types::Operation,
    rows::{errors::TableModelError, service::column_in},
};

/// Project columns without consulting a row; a repeated column remains repeated.
pub(crate) fn projected(
    kinds: &[CellKind],
    columns: &[ColumnIndex],
) -> Result<Vec<CellKind>, OperatingError> {
    columns
        .iter()
        .map(|column| {
            column_in(kinds, *column)
                .copied()
                .ok_or_else(|| TableModelError::NoSuchColumn { index: column.0 }.into())
        })
        .collect()
}

/// Carry the schema of an empty first instruction through the compiler's closed
/// row-continuation family without evaluating predicates or acquiring bindings.
pub(crate) fn continuation_kinds(
    operation: &Operation,
    input: &[CellKind],
) -> Result<Vec<CellKind>, OperatingError> {
    match operation {
        Operation::Select { kept, .. } if kept.is_empty() => Ok(input.to_vec()),
        Operation::Project {
            columns,
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
            ..
        } => projected(input, columns),
        _ => Err(OperatingError::WrongOperation),
    }
}
