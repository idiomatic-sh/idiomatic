//! The per-file input of a join after the files is narrowed in each file to the rows whose node key a value of the per-file step the
//! other input's key comes from meets; a key the plan follows only to a whole relation leaves the join as it is.

use super::service::narrow_lookups;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as Reads, JoinColumn, RepeatedRows, Side, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// The key after the files holds the values of a per-file step.
    KeyFromAPerFileStep,
    /// The key after the files is read from a whole relation.
    KeyFromARelation,
}

/// Whether the plan came back as the row expects it.
type Value = bool;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_per_file_input_is_narrowed_to_the_rows_the_per_file_origin_of_the_key_meets",
        ),
        word: Word::Succeeds,
        input: || Scenario::KeyFromAPerFileStep,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_key_read_from_a_whole_relation_leaves_the_join_as_it_is"),
        word: Word::Succeeds,
        input: || Scenario::KeyFromARelation,
        expect: Expect::Value(true),
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

fn join(left: Reads, right: Reads, output: Vec<JoinColumn>) -> Operation {
    Operation::Join {
        left,
        right,
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output,
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    }
}

/// The functions of each file, the ones after the files read from `crossing`, the comments of each file, and the comments of the
/// functions after the files.
fn before(crossing: Reads) -> Vec<PlanStep> {
    vec![
        step(
            Operation::Select {
                from: Reads::Relation(Relation::Function),
                comparisons: Vec::new(),
                kept: vec![ColumnIndex(0), ColumnIndex(2)],
            },
            StepScope::PerFile,
        ),
        step(
            Operation::Select {
                from: crossing,
                comparisons: Vec::new(),
                kept: Vec::new(),
            },
            StepScope::CrossFile,
        ),
        step(
            Operation::Select {
                from: Reads::Relation(Relation::Comment),
                comparisons: Vec::new(),
                kept: Vec::new(),
            },
            StepScope::PerFile,
        ),
        step(
            join(
                Reads::Step(StepId(1)),
                Reads::Step(StepId(2)),
                vec![column(Side::Left, 0), column(Side::Right, 1)],
            ),
            StepScope::CrossFile,
        ),
    ]
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let (steps, after, emits) = match scenario {
        Scenario::KeyFromAPerFileStep => {
            let steps = before(Reads::Step(StepId(0)));
            let mut after = steps.clone();
            after.truncate(3);
            after.push(step(
                Operation::Project {
                    from: Reads::Step(StepId(0)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Dropped,
                },
                StepScope::PerFile,
            ));
            after.push(step(
                join(
                    Reads::Step(StepId(2)),
                    Reads::Step(StepId(3)),
                    vec![column(Side::Left, 0), column(Side::Left, 1)],
                ),
                StepScope::PerFile,
            ));
            after.push(step(
                join(
                    Reads::Step(StepId(1)),
                    Reads::Step(StepId(4)),
                    vec![column(Side::Left, 0), column(Side::Right, 1)],
                ),
                StepScope::CrossFile,
            ));
            (steps, after, StepId(5))
        }
        Scenario::KeyFromARelation => {
            let steps = before(Reads::Relation(Relation::Function));
            (steps.clone(), steps, StepId(3))
        }
    };
    let planned = narrow_lookups(steps, StepId(3));
    Ok(planned.steps == after && planned.emitted == emits)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
