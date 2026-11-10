//! What the recogniser proves of one relation column, for the agreement controls of the crates above it.
use crate::domain::operator::types::StepPlace;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId};
use ryo_values::domain::plans::types::{Emit, Plan};

/// Whether the recogniser proves `column` of `relation` a stored number origin, the proof an ordered existence is
/// classified on.
#[must_use]
pub fn proves_stored_number(relation: Relation, column: ColumnIndex) -> bool {
    let plan = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps: vec![],
        comparison_origins: vec![],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
    };
    super::service::number_origins(&plan, Input::Relation(relation), column, StepPlace(0)).is_some()
}
