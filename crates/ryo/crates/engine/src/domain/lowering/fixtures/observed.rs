//! The plans of a rule whose plan hands rows over to steps after the files: an output another rule observes, which the
//! host adds to the producer's plan with the step it is read from on the plan's frontier.

use super::parts::{kept, plan, step};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, LiteralText, Operand, PlanComparison, QueryLiteral,
    RepeatedRows, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_plans::types::Plan;

/// A step that keeps every row of `from` and runs after the files.
fn after_the_files(from: Input) -> PlanStep {
    PlanStep {
        scope: StepScope::CrossFile,
        ..kept(from)
    }
}

/// The plans of the observed scenes; none for another name.
pub(super) fn observed_scene(scene: &str) -> Option<Plan> {
    Some(match scene {
        // The rule emits the elements (step 0). Another rule observes its words: step 1 selects them in the file, step 2
        // reads them after the files, and the frontier names step 1. Neither is a step of the rule's own rows.
        "a rule over the tree whose words another rule observes" => {
            let mut observed = plan(vec![
                kept(Input::Relation(Relation::SyntaxTree)),
                step(Operation::Select {
                    from: Input::Step(StepId(0)),
                    comparisons: vec![PlanComparison {
                        test: Test::Equal,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
                        origin: ComparisonOrigin::Unrecorded,
                    }],
                    kept: Vec::new(),
                }),
                after_the_files(Input::Step(StepId(1))),
            ]);
            observed.emit.step = StepId(0);
            observed.frontier = vec![Input::Step(StepId(1))];
            observed
        }
        // The rule's own rows are read after the files: its emit reaches a step that is not per-file, and whose rows (the
        // tree's elements alone) name no file, so they are rows of every file at once.
        "a rule that emits after the files" => {
            let mut late = plan(vec![
                kept(Input::Relation(Relation::SyntaxTree)),
                PlanStep {
                    operation: Operation::Project {
                        from: Input::Step(StepId(0)),
                        columns: vec![ColumnIndex(0)],
                        distinct: RepeatedRows::Dropped,
                    },
                    scope: StepScope::CrossFile,
                },
            ]);
            late.frontier = vec![Input::Step(StepId(0))];
            late
        }
        _ => return None,
    })
}
