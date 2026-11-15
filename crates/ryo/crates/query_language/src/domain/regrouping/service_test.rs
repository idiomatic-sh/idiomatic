//! A join after the files keyed on a column of its per-file input that says the file is made two joins, the per-file inputs joined in
//! each file, when one of the two is distinct on its keys; keyed on a column that does not say the file, or on one neither is distinct
//! on, it stays as it is.

use super::service::regroup_joins;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as Reads, JoinColumn, RepeatedRows, Side, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// The outer join is keyed on the file of the first join's per-file input, which the first join answers through its key.
    KeyedOnTheFile,
    /// The outer join is keyed on a name, which says nothing of the file.
    KeyedOnAName,
    /// The outer join is keyed on the file, but the files with a marker are not made distinct: a file is in them once per marker.
    KeyedOnAFileNeitherIsDistinctOn,
    /// The edges of the syntax tree, joined after the files with the edges under the same parent: every pair of siblings.
    SiblingsOnTheirParent,
    /// The edges of the syntax tree, joined after the files with the edge of their parent: one row at most for each.
    ParentOnItsOwnEdge,
}

/// The plan after the rewrite, and the step it emits.
type Value = (Vec<PlanStep>, StepId);
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_join_after_the_files_on_the_file_of_its_per_file_input_joins_the_per_file_inputs_in_each_file",
        ),
        word: Word::Succeeds,
        input: || Scenario::KeyedOnTheFile,
        expect: Expect::Value((Vec::new(), StepId(0))),
    },
    Case {
        name: CaseName("a_join_after_the_files_on_a_name_stays_as_it_is"),
        word: Word::Succeeds,
        input: || Scenario::KeyedOnAName,
        expect: Expect::Value((Vec::new(), StepId(0))),
    },
    Case {
        name: CaseName(
            "a_join_after_the_files_on_a_file_neither_per_file_input_is_distinct_on_stays_as_it_is",
        ),
        word: Word::Succeeds,
        input: || Scenario::KeyedOnAFileNeitherIsDistinctOn,
        expect: Expect::Value((Vec::new(), StepId(0))),
    },
    Case {
        name: CaseName("a_join_of_syntax_edges_on_their_shared_parent_stays_after_the_files"),
        word: Word::Succeeds,
        input: || Scenario::SiblingsOnTheirParent,
        expect: Expect::Value((Vec::new(), StepId(0))),
    },
    Case {
        name: CaseName(
            "a_join_of_syntax_edges_on_the_key_child_of_one_side_joins_them_in_each_file",
        ),
        word: Word::Succeeds,
        input: || Scenario::ParentOnItsOwnEdge,
        expect: Expect::Value((Vec::new(), StepId(0))),
    },
];

fn step(operation: Operation, scope: StepScope) -> PlanStep {
    PlanStep { operation, scope }
}

fn column(side: Side, index: u32) -> JoinColumn {
    JoinColumn {
        side,
        column: ColumnIndex(index),
    }
}

fn join(left: Reads, right: Reads, keys: (u32, u32), output: Vec<JoinColumn>) -> Operation {
    Operation::Join {
        left,
        right,
        left_on: vec![ColumnIndex(keys.0)],
        right_on: vec![ColumnIndex(keys.1)],
        output,
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    }
}

/// The candidate files after the files, the files with a marker and the functions of each file, and the functions of the candidate
/// files with a marker: `Join(Join(candidates, marked), functions)`, the second keyed on column `key` of the functions.
fn before(key: u32) -> Vec<PlanStep> {
    vec![
        step(
            Operation::Select {
                from: Reads::Relation(Relation::Function),
                comparisons: Vec::new(),
                kept: vec![ColumnIndex(2)],
            },
            StepScope::CrossFile,
        ),
        step(
            Operation::Project {
                from: Reads::Relation(Relation::Comment),
                columns: vec![ColumnIndex(2)],
                distinct: RepeatedRows::Dropped,
            },
            StepScope::PerFile,
        ),
        step(
            Operation::Select {
                from: Reads::Relation(Relation::Function),
                comparisons: Vec::new(),
                kept: vec![ColumnIndex(0), ColumnIndex(1), ColumnIndex(2)],
            },
            StepScope::PerFile,
        ),
        step(
            join(
                Reads::Step(StepId(0)),
                Reads::Step(StepId(1)),
                (0, 0),
                vec![column(Side::Left, 0)],
            ),
            StepScope::CrossFile,
        ),
        step(
            join(
                Reads::Step(StepId(3)),
                Reads::Step(StepId(2)),
                (0, key),
                vec![column(Side::Right, 0), column(Side::Right, 2)],
            ),
            StepScope::CrossFile,
        ),
    ]
}

/// The functions after the files, the edges under each function in each file and every edge in each file, and the edges under a
/// function joined with the edges on `key`: `Join(Join(functions, edges on child), edges)`, the second keyed on the parent of the
/// first edges and column `key` of the second.
fn edges(key: u32) -> Vec<PlanStep> {
    let every_edge = || Operation::Select {
        from: Reads::Relation(Relation::SyntaxChild),
        comparisons: Vec::new(),
        kept: Vec::new(),
    };
    vec![
        step(
            Operation::Select {
                from: Reads::Relation(Relation::Function),
                comparisons: Vec::new(),
                kept: vec![ColumnIndex(0)],
            },
            StepScope::CrossFile,
        ),
        step(every_edge(), StepScope::PerFile),
        step(every_edge(), StepScope::PerFile),
        step(
            join(
                Reads::Step(StepId(0)),
                Reads::Step(StepId(1)),
                (0, 1),
                vec![column(Side::Left, 0), column(Side::Right, 0)],
            ),
            StepScope::CrossFile,
        ),
        step(
            join(
                Reads::Step(StepId(3)),
                Reads::Step(StepId(2)),
                (1, key),
                vec![column(Side::Left, 0), column(Side::Right, 1)],
            ),
            StepScope::CrossFile,
        ),
    ]
}

/// The plan of `edges(1)` after the rewrite: the edges under a function joined in each file with the edge of their parent, and the
/// functions joined with that after the files.
fn edges_regrouped() -> Vec<PlanStep> {
    let mut steps = edges(1);
    steps.truncate(4);
    steps.push(step(
        join(
            Reads::Step(StepId(1)),
            Reads::Step(StepId(2)),
            (0, 1),
            vec![column(Side::Left, 1), column(Side::Right, 1)],
        ),
        StepScope::PerFile,
    ));
    steps.push(step(
        join(
            Reads::Step(StepId(0)),
            Reads::Step(StepId(4)),
            (0, 0),
            vec![column(Side::Left, 0), column(Side::Right, 1)],
        ),
        StepScope::CrossFile,
    ));
    steps
}

/// The plan of `before(2)` with the files with a marker kept once per marker, not once per file.
fn marked_per_marker() -> Vec<PlanStep> {
    let mut steps = before(2);
    if let Some(marked) = steps.get_mut(1) {
        marked.operation = Operation::Select {
            from: Reads::Relation(Relation::Comment),
            comparisons: Vec::new(),
            kept: vec![ColumnIndex(2)],
        };
    }
    steps
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let (steps, expected) = match scenario {
        Scenario::KeyedOnAFileNeitherIsDistinctOn => {
            (marked_per_marker(), (marked_per_marker(), StepId(4)))
        }
        Scenario::SiblingsOnTheirParent => (edges(0), (edges(0), StepId(4))),
        Scenario::ParentOnItsOwnEdge => (edges(1), (edges_regrouped(), StepId(5))),
        Scenario::KeyedOnTheFile | Scenario::KeyedOnAName => regrouped_by_hand(scenario),
    };
    let planned = regroup_joins(steps, StepId(4));
    // The row's value is empty when the plan is the one expected, and the plan that came back when it is not.
    Ok(if (planned.steps.clone(), planned.emitted) == expected {
        (Vec::new(), StepId(0))
    } else {
        (planned.steps, planned.emitted)
    })
}

/// The plan before the rewrite and the plan expected of it, for the rows over the files with a marker.
fn regrouped_by_hand(scenario: Input) -> (Vec<PlanStep>, Value) {
    let (key, expected) = match scenario {
        Scenario::KeyedOnTheFile => {
            let mut after = before(2);
            after.truncate(4);
            after.push(step(
                join(
                    Reads::Step(StepId(1)),
                    Reads::Step(StepId(2)),
                    (0, 2),
                    vec![
                        column(Side::Left, 0),
                        column(Side::Right, 0),
                        column(Side::Right, 2),
                    ],
                ),
                StepScope::PerFile,
            ));
            after.push(step(
                join(
                    Reads::Step(StepId(0)),
                    Reads::Step(StepId(4)),
                    (0, 0),
                    vec![column(Side::Right, 1), column(Side::Right, 2)],
                ),
                StepScope::CrossFile,
            ));
            (2, (after, StepId(5)))
        }
        Scenario::KeyedOnAName
        | Scenario::KeyedOnAFileNeitherIsDistinctOn
        | Scenario::SiblingsOnTheirParent
        | Scenario::ParentOnItsOwnEdge => (1, (before(1), StepId(4))),
    };
    (before(key), expected)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
