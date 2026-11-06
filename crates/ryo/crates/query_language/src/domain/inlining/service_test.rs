//! The `inlining` service's cases, on plans written by hand.

use ryo_relations::domain::relations::types::Relation;

use super::service::inline_views;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as Reads;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// What a row asks of the service.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// A per-file view that only reorders the columns of the tree is read in place by the step after it.
    APerFileViewIsReadInPlace,
    /// A view that runs after the files is left to be read as a table.
    ACrossFileViewIsLeft,
}

/// A number the answer is.
type Value = usize;
type Failure = ();
/// What a row asks of the service.
type Input = Scenario;

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

/// A view of the tree that keeps two columns in the other order, and a projection of its first column.
fn view_and_projection(scope: StepScope) -> Vec<PlanStep> {
    vec![
        PlanStep {
            operation: Operation::Select {
                from: Reads::Relation(Relation::SyntaxTree),
                comparisons: Vec::new(),
                kept: vec![column(2), column(0)],
            },
            scope,
        },
        PlanStep {
            operation: Operation::Project {
                from: Reads::Step(StepId(0)),
                columns: vec![column(0)],
                distinct: RepeatedRows::Dropped,
            },
            scope,
        },
    ]
}

fn read_in_place(scope: StepScope) -> usize {
    let inlined = inline_views(view_and_projection(scope));
    usize::from(
        matches!(&inlined[1].operation, Operation::Project { from: Reads::Relation(Relation::SyntaxTree), columns, .. } if columns == &[column(2)]),
    )
}

fn left_alone(scope: StepScope) -> usize {
    let steps = view_and_projection(scope);
    let inlined = inline_views(steps.clone());
    usize::from(inlined == steps)
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::APerFileViewIsReadInPlace => read_in_place(StepScope::PerFile),
        Scenario::ACrossFileViewIsLeft => left_alone(StepScope::CrossFile),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_per_file_view_of_the_tree_is_read_in_place_by_the_step_after_it"),
        word: Word::Succeeds,
        input: || Scenario::APerFileViewIsReadInPlace,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_view_that_runs_after_the_files_is_left_alone"),
        word: Word::Succeeds,
        input: || Scenario::ACrossFileViewIsLeft,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
