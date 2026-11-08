//! A column read from a relation through a join and a projection has the relation column's kind; a column a step made has none.

use super::service::column_kind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as Reads, JoinColumn, RepeatedRows, Side, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// The file of a function, through a join and a projection.
    AFileThroughAJoin,
    /// The count a grouping appended.
    ACount,
}

/// The kind found.
type Value = Option<CellKind>;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_column_read_through_a_join_and_a_projection_has_the_relation_column_kind",
        ),
        word: Word::Succeeds,
        input: || Scenario::AFileThroughAJoin,
        expect: Expect::Value(Some(CellKind::File)),
    },
    Case {
        name: CaseName("a_count_a_grouping_appended_has_no_kind"),
        word: Word::Succeeds,
        input: || Scenario::ACount,
        expect: Expect::Value(None),
    },
];

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let steps = vec![
        step(Operation::Join {
            left: Reads::Relation(Relation::Comment),
            right: Reads::Relation(Relation::Function),
            left_on: vec![ColumnIndex(2)],
            right_on: vec![ColumnIndex(2)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(2),
                },
            ],
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        }),
        step(Operation::Project {
            from: Reads::Step(StepId(0)),
            columns: vec![ColumnIndex(1)],
            distinct: RepeatedRows::Dropped,
        }),
        step(Operation::GroupCount {
            from: Reads::Step(StepId(1)),
            keys: vec![ColumnIndex(0)],
            distinct: None,
        }),
    ];
    Ok(match scenario {
        Scenario::AFileThroughAJoin => column_kind(&steps, Reads::Step(StepId(1)), ColumnIndex(0)),
        Scenario::ACount => column_kind(&steps, Reads::Step(StepId(2)), ColumnIndex(1)),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
