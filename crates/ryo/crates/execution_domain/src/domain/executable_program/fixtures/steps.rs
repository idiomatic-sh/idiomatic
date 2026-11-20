//! Shared compiled-plan builders for the program-interning fixtures.
use ryo_values::domain::observations::types::BoundObservationInput;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, LiteralText, Operand, PlanComparison,
    QueryLiteral, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_plans::types::{Emit, Plan};
use std::sync::Arc;

/// A comparison of a column with an operand.
pub(super) fn compared(test: Test, column: u32, right: Operand) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right,
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// A written text.
pub(super) fn text(written: &str) -> Operand {
    Operand::Literal(QueryLiteral::Text(LiteralText(written.into())))
}

/// A select of `from` by `comparisons`, keeping every column.
pub(super) fn select(from: StepInput, comparisons: Vec<PlanComparison>) -> Operation {
    Operation::Select {
        from,
        comparisons,
        kept: Vec::new(),
    }
}

/// A plan of `steps` that emits its last step and hands `frontier` to the steps after the files.
pub(super) fn plan(steps: Vec<PlanStep>, frontier: Vec<StepInput>) -> Arc<Plan> {
    bound_plan(steps, frontier, Vec::new())
}

/// A plan of `steps` with `observation_inputs`, that emits its last step and hands `frontier` to the steps after the files.
pub(super) fn bound_plan(
    steps: Vec<PlanStep>,
    frontier: Vec<StepInput>,
    observation_inputs: Vec<BoundObservationInput>,
) -> Arc<Plan> {
    Arc::new(Plan {
        observation_inputs,
        parameters: Vec::new(),
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        frontier,
        emit: Emit {
            step: StepId(u32::try_from(steps.len().saturating_sub(1)).unwrap_or(0)),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        steps,
    })
}

/// A step on each file.
pub(super) fn per_file(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}

/// The union of a rule's steps `left` and `right`.
pub(super) fn union_of(left: u32, right: u32) -> PlanStep {
    per_file(Operation::Union {
        left: StepInput::Step(StepId(left)),
        right: StepInput::Step(StepId(right)),
    })
}
