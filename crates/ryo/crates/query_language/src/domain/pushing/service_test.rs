//! The `pushing` service's cases, on plans written by hand.

use ryo_relations::domain::relations::types::Relation;

use super::service::push_lookups;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as Reads;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// What a row asks of the service.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// A lookup by the parent of the edges of a join of edges and elements is two joins, the edges first.
    ALookupByTheParentIsPushed,
    /// A lookup by another column of the join is left as it is.
    ALookupByAnotherColumnIsLeft,
}

/// A number the answer is.
type Value = usize;
type Failure = ();
/// What a row asks of the service.
type Input = Scenario;

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::CrossFile,
    }
}

/// The functions, the join of the edges of the tree with its elements, and a lookup of the first into the
/// second by the column `asked` of the join.
fn lookup(asked: u32) -> Vec<PlanStep> {
    vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Join {
            left: Reads::Relation(Relation::SyntaxChild),
            right: Reads::Relation(Relation::SyntaxTree),
            left_on: vec![column(1)],
            right_on: vec![column(0)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: column(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: column(0),
                },
            ],
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        }),
        step(Operation::Join {
            left: Reads::Step(StepId(0)),
            right: Reads::Step(StepId(1)),
            left_on: vec![column(0)],
            right_on: vec![column(asked)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: column(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: column(1),
                },
            ],
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        }),
    ]
}

fn pushed_lookup() -> usize {
    let planned = push_lookups(lookup(0), StepId(2));
    usize::from(
        planned.steps.len() == 4
            && planned.emitted == StepId(3)
            && matches!(
                &planned.steps[2].operation,
                Operation::Join {
                    left: Reads::Step(StepId(0)),
                    right: Reads::Relation(Relation::SyntaxChild),
                    ..
                }
            )
            && matches!(
                &planned.steps[3].operation,
                Operation::Join {
                    left: Reads::Step(StepId(2)),
                    right: Reads::Relation(Relation::SyntaxTree),
                    ..
                }
            ),
    )
}

fn left_lookup() -> usize {
    let steps = lookup(1);
    let planned = push_lookups(steps.clone(), StepId(2));
    usize::from(planned.steps == steps && planned.emitted == StepId(2))
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::ALookupByTheParentIsPushed => pushed_lookup(),
        Scenario::ALookupByAnotherColumnIsLeft => left_lookup(),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_lookup_by_the_parent_of_the_edges_is_two_joins_the_edges_first"),
        word: Word::Succeeds,
        input: || Scenario::ALookupByTheParentIsPushed,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_lookup_by_another_column_is_left_alone"),
        word: Word::Succeeds,
        input: || Scenario::ALookupByAnotherColumnIsLeft,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
