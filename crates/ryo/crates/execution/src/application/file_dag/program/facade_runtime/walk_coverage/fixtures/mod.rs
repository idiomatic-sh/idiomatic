//! Plans use the normal operator, graph and physical-program compilers.
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::plan_parts::types::StepScope;

pub(super) mod catalog;
pub(super) mod meaning;
pub(super) mod operations;

pub(super) fn plan(operation: Operation, scope: StepScope) -> Plan {
    Plan {
        observation_inputs: vec![],
        parameters: vec![],
        comparison_origins: vec![],
        observations: vec![],
        steps: vec![PlanStep { operation, scope }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
    }
}
