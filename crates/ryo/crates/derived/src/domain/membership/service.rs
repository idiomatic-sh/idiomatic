//! Duplicate-insensitive membership; no domain cache identity or admission policy.
use super::types::{DerivedMembership, MarkSpace};
use crate::domain::fault::types::DerivedFault;
use crate::domain::positions::types::Mark;
use crate::domain::rows::types::RowCells;
use crate::domain::sharing::types::shared_marks::value::SharedMarks;
use ryo_relations::domain::relations::{
    service::{node_file, node_position},
    types::Value,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;

/// Collects the values of a column and marks them in the space.
pub(crate) fn build(
    rows: &dyn RowCells,
    column: ColumnIndex,
    space: MarkSpace<'_>,
) -> Result<DerivedMembership, DerivedFault> {
    let mut values = hashbrown::HashSet::new();
    for at in 0..rows.count().0 {
        let value = rows
            .cell(RowIndex(at), column)
            .ok_or(DerivedFault::ShapeMismatch)?;
        let _new = values.insert(value);
    }
    Ok(match space {
        MarkSpace::Values => DerivedMembership::Values(values),
        MarkSpace::Dense(vertices) => DerivedMembership::Dense(SharedMarks::new(
            vertices
                .iter()
                .map(|value| {
                    if values.contains(value) {
                        Mark::Member
                    } else {
                        Mark::Absent
                    }
                })
                .collect(),
        )),
        MarkSpace::Native { file, elements } => {
            let mut marks = vec![Mark::Absent; elements.0];
            for value in values {
                if let Value::Node(node) = value {
                    if node_file(node) == file {
                        if let Some(mark) = marks.get_mut(node_position(node).0 as usize) {
                            *mark = Mark::Member;
                        }
                    }
                }
            }
            DerivedMembership::Dense(SharedMarks::new(marks))
        }
    })
}
