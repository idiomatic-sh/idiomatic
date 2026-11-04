//! The `lowering` service's cases, on values alone: which steps of a plan are lowered.

use ryo_relations::domain::relations::types::Relation;

use super::service::lower;
use super::types::Counting;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};

/// The plan a row lowers.
type Input = Plan;
/// How many steps are lowered, and how many count the rows of a lowered step.
type Value = (usize, usize);
type Failure = ();

/// A plan of these steps, all per file, emitting the last.
fn plan_of(steps: Vec<Operation>) -> Plan {
    let last = steps.len().saturating_sub(1);
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: steps
            .into_iter()
            .map(|operation| PlanStep {
                operation,
                scope: StepScope::PerFile,
            })
            .collect(),
        emit: Emit {
            step: StepId(u32::try_from(last).unwrap_or(0)),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

fn select_of(relation: Relation) -> Operation {
    Operation::Select {
        from: PlanInput::Relation(relation),
        comparisons: Vec::new(),
        kept: Vec::new(),
    }
}

fn the_tree() -> Plan {
    plan_of(vec![select_of(Relation::SyntaxTree)])
}

fn a_stored_relation() -> Plan {
    plan_of(vec![select_of(Relation::Function)])
}

fn the_tree_joined_to_a_stored_relation() -> Plan {
    plan_of(vec![
        select_of(Relation::SyntaxTree),
        Operation::Join {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Relation(Relation::FilesystemLineCount),
            left_on: vec![ColumnIndex(3)],
            right_on: vec![ColumnIndex(0)],
            output: vec![JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            }],
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        },
    ])
}

fn the_tree_with_its_children_counted() -> Plan {
    plan_of(vec![
        select_of(Relation::SyntaxTree),
        Operation::Join {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Relation(Relation::SyntaxChild),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(1),
                },
            ],
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        },
        Operation::GroupCount {
            from: PlanInput::Step(StepId(1)),
            keys: vec![ColumnIndex(0)],
            distinct: None,
        },
    ])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_select_of_the_tree_is_lowered"),
        word: Word::Succeeds,
        input: the_tree,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_select_of_a_stored_relation_runs_as_an_operator"),
        word: Word::Succeeds,
        input: a_stored_relation,
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName(
            "a_join_with_a_stored_relation_runs_as_an_operator_and_what_it_reads_is_lowered",
        ),
        word: Word::Succeeds,
        input: the_tree_joined_to_a_stored_relation,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_count_of_lowered_rows_is_counted_from_them"),
        word: Word::Succeeds,
        input: the_tree_with_its_children_counted,
        expect: Expect::Value((2, 1)),
    },
];

fn subject(plan: &Input) -> Value {
    let lowering = lower(plan);
    (
        lowering
            .shapes
            .iter()
            .filter(|shape| shape.is_some())
            .count(),
        lowering
            .counting
            .iter()
            .filter(|each| **each == Counting::Counts)
            .count(),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(&input)));
}
