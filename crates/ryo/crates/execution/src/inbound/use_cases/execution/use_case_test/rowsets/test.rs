//! Rowset contracts use independently authored host populations and the actual public owner.
use super::super::super::fixtures;
use super::super::super::fixtures::requests::numbers;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Relation, Value as Cell};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        hosted::types::HostRelation,
        plan_parts::types::{
            ColumnIndex, HostedNumber, Input as PlanInput, RepeatedRows, StepId, StepScope,
        },
        plans::types::{Emit, Operation, Plan, PlanStep},
        rows::errors::RowsError,
    },
};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    ProjectDistinct,
    ProjectKept,
    ProjectZeroKept,
    ProjectZeroDistinct,
    ProjectMissing,
    Union,
    UnionZero,
    UnionUnlike,
    Intersection,
    IntersectionUnlike,
    Difference,
    DifferenceUnlike,
    Antijoin,
    AntijoinColumn,
    AntijoinHeld,
    AntijoinChangedRight,
    AntijoinAbsent,
    Count,
    CountMissing,
}
type Input = Scenario;
type Value = (usize, Cow<'static, [i64]>);
type Failure = OperatingError;

fn authored(scenario: Input) -> (Plan, Vec<HostRelation>) {
    let left = PlanInput::Step(StepId(0));
    let missing = matches!(scenario, Scenario::CountMissing);
    let right = if missing {
        PlanInput::Relation(Relation::Function)
    } else {
        PlanInput::Step(StepId(1))
    };
    let project = matches!(
        scenario,
        Scenario::ProjectDistinct
            | Scenario::ProjectKept
            | Scenario::ProjectZeroKept
            | Scenario::ProjectZeroDistinct
            | Scenario::ProjectMissing
    );
    let mut hosts = if project || matches!(scenario, Scenario::Count) {
        vec![numbers("left", 2, &[&[1, 10], &[1, 20], &[2, 30]])]
    } else if matches!(scenario, Scenario::UnionZero) {
        vec![numbers("left", 0, &[&[], &[]]), numbers("right", 0, &[&[]])]
    } else if matches!(scenario, Scenario::Antijoin | Scenario::AntijoinColumn) {
        vec![
            numbers("left", 2, &[&[1, 10], &[2, 20]]),
            numbers("right", 1, &[&[1]]),
        ]
    } else if matches!(scenario, Scenario::AntijoinChangedRight) {
        vec![numbers("left", 1, &[&[2]]), numbers("right", 1, &[&[1]])]
    } else if matches!(scenario, Scenario::AntijoinHeld | Scenario::AntijoinAbsent) {
        vec![
            numbers(
                "left",
                1,
                if matches!(scenario, Scenario::AntijoinHeld) {
                    &[&[2]]
                } else {
                    &[&[9]]
                },
            ),
            numbers("right", 1, &[&[1], &[2]]),
        ]
    } else {
        vec![
            numbers("left", 1, &[&[1], &[2]]),
            numbers("right", 1, &[&[2], &[3]]),
        ]
    };
    if matches!(
        scenario,
        Scenario::UnionUnlike | Scenario::IntersectionUnlike | Scenario::DifferenceUnlike
    ) {
        hosts[1] = numbers("right", 2, &[&[1, 2]]);
    }
    if missing {
        hosts.clear();
    }
    let mut steps: Vec<_> = (0..hosts.len())
        .map(|index| PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Hosted {
                relation: HostedNumber(u32::try_from(index).expect("a hosted number fits in u32")),
            },
        })
        .collect();
    if missing {
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![],
            },
        });
    }
    let operation = match scenario {
        Scenario::ProjectDistinct
        | Scenario::ProjectKept
        | Scenario::ProjectZeroKept
        | Scenario::ProjectZeroDistinct
        | Scenario::ProjectMissing => Operation::Project {
            from: left,
            columns: if matches!(
                scenario,
                Scenario::ProjectZeroKept | Scenario::ProjectZeroDistinct
            ) {
                vec![]
            } else {
                vec![ColumnIndex(
                    if matches!(scenario, Scenario::ProjectMissing) {
                        5
                    } else {
                        0
                    },
                )]
            },
            distinct: if matches!(scenario, Scenario::ProjectKept | Scenario::ProjectZeroKept) {
                RepeatedRows::Kept
            } else {
                RepeatedRows::Dropped
            },
        },
        Scenario::Union | Scenario::UnionZero | Scenario::UnionUnlike => {
            Operation::Union { left, right }
        }
        Scenario::Intersection | Scenario::IntersectionUnlike => {
            Operation::Intersection { left, right }
        }
        Scenario::Difference | Scenario::DifferenceUnlike => Operation::Difference { left, right },
        Scenario::Antijoin
        | Scenario::AntijoinColumn
        | Scenario::AntijoinHeld
        | Scenario::AntijoinChangedRight
        | Scenario::AntijoinAbsent => Operation::Antijoin {
            left,
            right,
            left_on: vec![ColumnIndex(
                if matches!(scenario, Scenario::AntijoinColumn) {
                    4
                } else {
                    0
                },
            )],
            right_on: vec![ColumnIndex(0)],
        },
        Scenario::Count | Scenario::CountMissing => Operation::GroupCount {
            from: left,
            keys: vec![ColumnIndex(0)],
            distinct: None,
        },
    };
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation,
    });
    let emit = Emit {
        step: StepId(u32::try_from(steps.len() - 1).expect("a step number fits in u32")),
        subject: ColumnIndex(0),
        key: vec![],
        headings: vec![],
    };
    (
        Plan {
            parameters: vec![],
            observation_inputs: vec![],
            observations: vec![],
            comparison_origins: vec![],
            frontier: vec![],
            steps,
            emit,
        },
        hosts,
    )
}
fn subject(input: Input) -> Result<Value, Failure> {
    let (plan, hosts) = authored(input);
    let table = fixtures::requests::published(plan, vec![], hosts, &mut Symbols::default())?;
    let values = table
        .cells
        .iter()
        .map(|cell| match cell {
            Cell::Number(number) => number.0,
            Cell::Node(_)
            | Cell::Symbol(_)
            | Cell::Name(_)
            | Cell::File(_)
            | Cell::Span(_)
            | Cell::Flag(_) => panic!("numeric authored contract must keep numeric output cells"),
        })
        .collect();
    Ok((table.count.0, Cow::Owned(values)))
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("project_keeps_one_column_and_drops_repeats"),
        word: Word::Succeeds,
        input: || Scenario::ProjectDistinct,
        expect: Expect::Value((2, Cow::Borrowed(&[1, 2]))),
    },
    Case {
        name: CaseName("project_keeps_repeats_when_asked_to"),
        word: Word::Succeeds,
        input: || Scenario::ProjectKept,
        expect: Expect::Value((3, Cow::Borrowed(&[1, 1, 2]))),
    },
    Case {
        name: CaseName("zero_column_projection_keeps_all_three_rows"),
        word: Word::Succeeds,
        input: || Scenario::ProjectZeroKept,
        expect: Expect::Value((3, Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("zero_column_distinct_projection_keeps_one_row"),
        word: Word::Succeeds,
        input: || Scenario::ProjectZeroDistinct,
        expect: Expect::Value((1, Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("project_refuses_its_missing_column"),
        word: Word::Refuses,
        input: || Scenario::ProjectMissing,
        expect: Expect::Error(OperatingError::Table(RowsError::NoSuchColumn { index: 5 })),
    },
    Case {
        name: CaseName("union_keeps_exact_distinct_rows"),
        word: Word::Succeeds,
        input: || Scenario::Union,
        expect: Expect::Value((3, Cow::Borrowed(&[1, 2, 3]))),
    },
    Case {
        name: CaseName("zero_column_union_keeps_one_distinct_empty_row"),
        word: Word::Succeeds,
        input: || Scenario::UnionZero,
        expect: Expect::Value((1, Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("intersection_keeps_exact_shared_rows"),
        word: Word::Succeeds,
        input: || Scenario::Intersection,
        expect: Expect::Value((1, Cow::Borrowed(&[2]))),
    },
    Case {
        name: CaseName("difference_keeps_exact_left_only_rows"),
        word: Word::Succeeds,
        input: || Scenario::Difference,
        expect: Expect::Value((1, Cow::Borrowed(&[1]))),
    },
    Case {
        name: CaseName("union_refuses_unlike_columns"),
        word: Word::Refuses,
        input: || Scenario::UnionUnlike,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("intersection_refuses_unlike_columns"),
        word: Word::Refuses,
        input: || Scenario::IntersectionUnlike,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("difference_refuses_unlike_columns"),
        word: Word::Refuses,
        input: || Scenario::DifferenceUnlike,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("antijoin_keeps_exact_unmatched_left_row"),
        word: Word::Succeeds,
        input: || Scenario::Antijoin,
        expect: Expect::Value((1, Cow::Borrowed(&[2, 20]))),
    },
    Case {
        name: CaseName("antijoin_refuses_exact_missing_key_column"),
        word: Word::Refuses,
        input: || Scenario::AntijoinColumn,
        expect: Expect::Error(OperatingError::Table(RowsError::NoSuchColumn { index: 4 })),
    },
    Case {
        name: CaseName("a_key_the_right_holds_is_not_absent"),
        word: Word::Succeeds,
        input: || Scenario::AntijoinHeld,
        expect: Expect::Value((0, Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("a_key_the_right_lacks_is_absent"),
        word: Word::Succeeds,
        input: || Scenario::AntijoinAbsent,
        expect: Expect::Value((1, Cow::Borrowed(&[9]))),
    },
    Case {
        name: CaseName("count_keeps_one_row_per_group_and_exact_sizes"),
        word: Word::Succeeds,
        input: || Scenario::Count,
        expect: Expect::Value((2, Cow::Borrowed(&[1, 2, 2, 1]))),
    },
    Case {
        name: CaseName("count_requires_its_real_provider"),
        word: Word::Refuses,
        input: || Scenario::CountMissing,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("changing_the_second_population_changes_exact_antijoin_output"),
        word: Word::Succeeds,
        input: || Scenario::AntijoinChangedRight,
        expect: Expect::Value((1, Cow::Borrowed(&[2]))),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
