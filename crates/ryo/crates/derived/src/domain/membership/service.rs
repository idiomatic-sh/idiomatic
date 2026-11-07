//! Duplicate-insensitive membership; no domain cache identity or admission policy.
use crate::domain::contract::types::{
    DerivedFault, Membership, MembershipSpace, RowCells, SharedMarks,
};
use ryo_relations::domain::relations::{
    service::{node_file, node_position},
    types::Value,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;
pub(crate) fn build(
    rows: &dyn RowCells,
    column: ColumnIndex,
    space: MembershipSpace<'_>,
) -> Result<Membership, DerivedFault> {
    let mut values = hashbrown::HashSet::new();
    for at in 0..rows.count() {
        let value = rows.cell(at, column).ok_or(DerivedFault::ShapeMismatch)?;
        let _new = values.insert(value);
    }
    Ok(match space {
        MembershipSpace::Values => Membership::Values(values),
        MembershipSpace::Dense(vertices) => Membership::Dense(SharedMarks(Arc::new(
            vertices
                .iter()
                .map(|value| values.contains(value))
                .collect(),
        ))),
        MembershipSpace::Native { file, elements } => {
            let mut marks = vec![false; elements];
            for value in values {
                if let Value::Node(node) = value {
                    if node_file(node) == file {
                        if let Some(mark) = marks.get_mut(node_position(node).0 as usize) {
                            *mark = true;
                        }
                    }
                }
            }
            Membership::Dense(SharedMarks(Arc::new(marks)))
        }
    })
}
