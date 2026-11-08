//! A join after the files keyed on a column of its per-file input that says the file is made two joins, the per-file inputs joined in
//! each file; keyed on a column that does not say the file, it stays as it is.

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
}

/// The plan after the rewrite, and the step it emits.
type Value = (Vec<PlanStep>, StepId);
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_join_after_the_files_on_the_file_of_its_per_file_input_joins_the_per_file_inputs_in_each_file"),
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

fn subject(scenario: Input) -> Result<Value, Failure> {
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
        Scenario::KeyedOnAName => (1, (before(1), StepId(4))),
    };
    let planned = regroup_joins(before(key), StepId(4));
    // The row's value is empty when the plan is the one expected, and the plan that came back when it is not.
    Ok(if (planned.steps.clone(), planned.emitted) == expected {
        (Vec::new(), StepId(0))
    } else {
        (planned.steps, planned.emitted)
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
