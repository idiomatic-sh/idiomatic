//! The support of the `ordered_partial` cases.
use super::super::route_controls::fixtures::ordered_strategy_fixture;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, Test};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId, StepScope},
    plans::types::{Operation, Plan, PlanStep},
};

/// The first location of each line, by byte: an antijoin of the locations over the self join of each line's locations. With `kept`
/// empty the engine recognises it as an ordered existence; keeping `[0, 1]`, the same meaning, declines the shape.
pub(super) fn first_of_each_line(kept: Vec<ColumnIndex>) -> Plan {
    use ryo_values::domain::plan_parts::types::{ComparisonOrigin, JoinColumn, RepeatedRows, Side};
    let (line, byte) = (ColumnIndex(3), ColumnIndex(5));
    let mut plan = ordered_strategy_fixture().0;
    plan.steps = [
        Operation::Select {
            from: PlanInput::Relation(Relation::SourceLocation),
            comparisons: vec![],
            kept: vec![],
        },
        Operation::Join {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Step(StepId(0)),
            left_on: vec![line],
            right_on: vec![line],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: byte,
                },
                JoinColumn {
                    side: Side::Right,
                    column: byte,
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        },
        Operation::Select {
            from: PlanInput::Step(StepId(1)),
            kept,
            comparisons: vec![PlanComparison {
                test: Test::LessThan,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Column(ColumnIndex(1)),
                origin: ComparisonOrigin::Unrecorded,
            }],
        },
        Operation::Project {
            from: PlanInput::Step(StepId(2)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        },
        Operation::Antijoin {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Step(StepId(3)),
            left_on: vec![byte],
            right_on: vec![ColumnIndex(0)],
        },
    ]
    .into_iter()
    .map(|operation| PlanStep {
        scope: StepScope::PerFile,
        operation,
    })
    .collect();
    plan.emit.step = StepId(4);
    plan
}
