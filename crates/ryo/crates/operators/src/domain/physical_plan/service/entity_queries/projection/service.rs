//! Bind each column once, preserving the width and order specified by the physical operation.

use crate::domain::entity_queries::types::EntityColumn;
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(in super::super) fn columns(
    fields: &[EntityColumn],
    kept: &[ColumnIndex],
) -> Option<Vec<EntityColumn>> {
    kept.iter()
        .map(|&column| fields.get(usize::from(column)).copied())
        .collect()
}
