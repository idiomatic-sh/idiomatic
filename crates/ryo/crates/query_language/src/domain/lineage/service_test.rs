//! A column read from a relation through a join and a projection has the relation column's kind; a column a step made has none.

use super::service::{column_kind, file_lineage, value_origins};
use super::types::LineageMemo;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as Reads, JoinColumn, RepeatedRows, Side, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use std::borrow::Cow;

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// The file of a function, through a join and a projection.
    AFileThroughAJoin,
    /// The count a grouping appended.
    ACount,
}

/// What a row asks: the kind of one scenario's column, or a walk of a ladder at a depth and at twice it.
#[derive(Clone, Copy)]
enum Ask {
    /// The kind of the scenario's column.
    Kind(Scenario),
    /// The lineage of the last step of a ladder of so many per-file joins, each of the step before with itself.
    Lineage(usize),
    /// The per-file origins of the first column of the last step of a ladder of so many unions after the files, each of the step
    /// before with itself.
    Origins(usize),
}

/// What a row answers.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// The kind found.
    Kind(Option<CellKind>),
    /// How much of a ladder a walk followed at a depth and at twice it, beside how many steps each ladder has.
    Counted(Cow<'static, str>),
}

type Value = Answer;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Ask;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_column_read_through_a_join_and_a_projection_has_the_relation_column_kind",
        ),
        word: Word::Succeeds,
        input: || Ask::Kind(Scenario::AFileThroughAJoin),
        expect: Expect::Value(Answer::Kind(Some(CellKind::File))),
    },
    Case {
        name: CaseName("a_count_a_grouping_appended_has_no_kind"),
        word: Word::Succeeds,
        input: || Ask::Kind(Scenario::ACount),
        expect: Expect::Value(Answer::Kind(None)),
    },
    Case {
        name: CaseName("the_lineage_of_a_ladder_of_joins_follows_each_column_of_a_step_once"),
        word: Word::Succeeds,
        input: || Ask::Lineage(16),
        expect: Expect::Value(Answer::Counted(Cow::Borrowed(
            "16 levels, 17 steps: 18 columns followed, 1 lineage found, one file true; 32 levels, 33 steps: 34 columns followed, 1 lineage found, one file true",
        ))),
    },
    Case {
        name: CaseName("the_origins_under_a_ladder_of_unions_follow_each_step_once"),
        word: Word::Succeeds,
        input: || Ask::Origins(16),
        expect: Expect::Value(Answer::Counted(Cow::Borrowed(
            "16 levels, 17 steps: 17 columns followed, Some(1) origins; 32 levels, 33 steps: 33 columns followed, Some(1) origins",
        ))),
    },
];

/// The step a ladder reads at its foot, then `levels` steps in `scope`, each made of the step before by `level`.
fn ladder(levels: usize, scope: StepScope, level: fn(Reads) -> Operation) -> Vec<PlanStep> {
    let mut steps = vec![step(Operation::Select {
        from: Reads::Relation(Relation::Function),
        comparisons: Vec::new(),
        kept: vec![ColumnIndex(2), ColumnIndex(0)],
    })];
    for _ in 0..levels {
        let before = last(&steps);
        steps.push(PlanStep {
            written: Default::default(),
            operation: level(before),
            scope,
        });
    }
    steps
}

/// The last step of `steps`.
fn last(steps: &[PlanStep]) -> Reads {
    Reads::Step(StepId(
        u32::try_from(steps.len().saturating_sub(1)).unwrap_or(u32::MAX),
    ))
}

/// A join of `before` with itself on its first column, keeping the first column of each side.
fn joined(before: Reads) -> Operation {
    Operation::Join {
        left: before,
        right: before,
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(0),
            },
        ],
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    }
}

/// A union of `before` with itself.
fn united(before: Reads) -> Operation {
    Operation::Union {
        left: before,
        right: before,
    }
}

/// How many columns of steps the lineage of the last step of a per-file ladder of joins followed a kind for, at `levels` levels.
fn lineage_followed(levels: usize) -> String {
    let steps = ladder(levels, StepScope::PerFile, joined);
    let mut memo = LineageMemo::default();
    let lineage = file_lineage(&steps, &mut memo, last(&steps));
    format!(
        "{levels} levels, {} steps: {} columns followed, {} lineage found, one file {}",
        steps.len(),
        memo.kinds_followed.0,
        memo.lineages_followed.0,
        matches!(
            lineage,
            ryo_values::domain::lineage::types::Lineage::OneFile(_)
        ),
    )
}

/// How many columns of steps the origins of the first column of the last step of a ladder of unions were followed for.
fn origins_followed(levels: usize) -> String {
    let steps = ladder(levels, StepScope::CrossFile, united);
    let mut memo = LineageMemo::default();
    let origins = value_origins(&steps, &mut memo, last(&steps), ColumnIndex(0));
    format!(
        "{levels} levels, {} steps: {} columns followed, {:?} origins",
        steps.len(),
        memo.origins_followed.0,
        origins.map(|found| found.len()),
    )
}

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        written: Default::default(),
        operation,
        scope: StepScope::PerFile,
    }
}

fn subject(ask: Input) -> Result<Value, Failure> {
    let scenario = match ask {
        Ask::Kind(scenario) => scenario,
        Ask::Lineage(levels) => {
            return Ok(Answer::Counted(Cow::Owned(format!(
                "{}; {}",
                lineage_followed(levels),
                lineage_followed(levels.saturating_mul(2))
            ))));
        }
        Ask::Origins(levels) => {
            return Ok(Answer::Counted(Cow::Owned(format!(
                "{}; {}",
                origins_followed(levels),
                origins_followed(levels.saturating_mul(2))
            ))));
        }
    };
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
    Ok(Answer::Kind(match scenario {
        Scenario::AFileThroughAJoin => column_kind(
            &steps,
            &mut LineageMemo::default(),
            Reads::Step(StepId(1)),
            ColumnIndex(0),
        ),
        Scenario::ACount => column_kind(
            &steps,
            &mut LineageMemo::default(),
            Reads::Step(StepId(2)),
            ColumnIndex(1),
        ),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
