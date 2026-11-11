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

/// One hosted population: its column count and rows.
type Hosted = (usize, &'static [&'static [i64]]);

/// The rowset operator a row runs over hosted populations 0 (left) and 1 (right).
#[derive(Clone, Copy, Debug)]
enum SetOp {
    /// Projects the left on `columns`, dropping repeated rows or keeping them.
    Project {
        columns: &'static [u32],
        distinct: bool,
    },
    Union,
    Intersection,
    Difference,
    /// The left rows whose column `left_on` no right row's column 0 holds.
    Antijoin {
        left_on: u32,
    },
    /// One row per key in the left's column 0, with its group's size.
    Count,
}

/// The hosted populations and the operator. With no population there is no hosted provider: the
/// left is a select of the function relation, and the right is the function relation.
#[derive(Clone, Copy, Debug)]
struct Rowsets {
    hosts: &'static [Hosted],
    operator: SetOp,
}

/// What the operator publishes: its rows, and every cell in order.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Published {
    rows: usize,
    cells: Cow<'static, [i64]>,
}

type Input = Rowsets;
type Value = Published;
type Failure = OperatingError;

/// The operator over the populations.
const fn over(hosts: &'static [Hosted], operator: SetOp) -> Rowsets {
    Rowsets { hosts, operator }
}

/// One two-column population keyed 1, 1 and 2.
const ONE_KEYED: &[Hosted] = &[(2, &[&[1, 10], &[1, 20], &[2, 30]])];
/// The sets {1, 2} and {2, 3}.
const TWO_SETS: &[Hosted] = &[(1, &[&[1], &[2]]), (1, &[&[2], &[3]])];
/// The set {1, 2} and a two-column right.
const UNLIKE: &[Hosted] = &[(1, &[&[1], &[2]]), (2, &[&[1, 2]])];
/// Two keyed rows and a right holding key 1.
const KEYED_BY_ONE: &[Hosted] = &[(2, &[&[1, 10], &[2, 20]]), (1, &[&[1]])];

/// The plan of the hosted populations followed by the operator, and the populations.
fn authored(rowsets: Input) -> (Plan, Vec<HostRelation>) {
    let left = PlanInput::Step(StepId(0));
    let names = ["left", "right"];
    let hosts: Vec<HostRelation> = rowsets
        .hosts
        .iter()
        .zip(names)
        .map(|(&(width, rows), name)| numbers(name, width, rows))
        .collect();
    let right = if hosts.is_empty() {
        PlanInput::Relation(Relation::Function)
    } else {
        PlanInput::Step(StepId(1))
    };
    let mut steps: Vec<_> = (0..hosts.len())
        .map(|index| PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Hosted {
                relation: HostedNumber(u32::try_from(index).expect("a hosted number fits in u32")),
            },
        })
        .collect();
    if hosts.is_empty() {
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![],
            },
        });
    }
    let operation = match rowsets.operator {
        SetOp::Project { columns, distinct } => Operation::Project {
            from: left,
            columns: columns.iter().copied().map(ColumnIndex).collect(),
            distinct: if distinct {
                RepeatedRows::Dropped
            } else {
                RepeatedRows::Kept
            },
        },
        SetOp::Union => Operation::Union { left, right },
        SetOp::Intersection => Operation::Intersection { left, right },
        SetOp::Difference => Operation::Difference { left, right },
        SetOp::Antijoin { left_on } => Operation::Antijoin {
            left,
            right,
            left_on: vec![ColumnIndex(left_on)],
            right_on: vec![ColumnIndex(0)],
        },
        SetOp::Count => Operation::GroupCount {
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
/// Publishes the operator over the populations.
fn subject(rowsets: Input) -> Result<Value, Failure> {
    let (plan, hosts) = authored(rowsets);
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
    Ok(Published {
        rows: table.count.0,
        cells: Cow::Owned(values),
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("project_keeps_one_column_and_drops_repeats"),
        word: Word::Succeeds,
        input: || {
            over(
                ONE_KEYED,
                SetOp::Project {
                    columns: &[0],
                    distinct: true,
                },
            )
        },
        expect: Expect::Value(Published {
            rows: 2,
            cells: Cow::Borrowed(&[1, 2]),
        }),
    },
    Case {
        name: CaseName("project_keeps_repeats_when_asked_to"),
        word: Word::Succeeds,
        input: || {
            over(
                ONE_KEYED,
                SetOp::Project {
                    columns: &[0],
                    distinct: false,
                },
            )
        },
        expect: Expect::Value(Published {
            rows: 3,
            cells: Cow::Borrowed(&[1, 1, 2]),
        }),
    },
    Case {
        name: CaseName("zero_column_projection_keeps_all_three_rows"),
        word: Word::Succeeds,
        input: || {
            over(
                ONE_KEYED,
                SetOp::Project {
                    columns: &[],
                    distinct: false,
                },
            )
        },
        expect: Expect::Value(Published {
            rows: 3,
            cells: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("zero_column_distinct_projection_keeps_one_row"),
        word: Word::Succeeds,
        input: || {
            over(
                ONE_KEYED,
                SetOp::Project {
                    columns: &[],
                    distinct: true,
                },
            )
        },
        expect: Expect::Value(Published {
            rows: 1,
            cells: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("project_refuses_its_missing_column"),
        word: Word::Refuses,
        input: || {
            over(
                ONE_KEYED,
                SetOp::Project {
                    columns: &[5],
                    distinct: true,
                },
            )
        },
        expect: Expect::Error(OperatingError::Table(RowsError::NoSuchColumn { index: 5 })),
    },
    Case {
        name: CaseName("union_keeps_exact_distinct_rows"),
        word: Word::Succeeds,
        input: || over(TWO_SETS, SetOp::Union),
        expect: Expect::Value(Published {
            rows: 3,
            cells: Cow::Borrowed(&[1, 2, 3]),
        }),
    },
    Case {
        name: CaseName("zero_column_union_keeps_one_distinct_empty_row"),
        word: Word::Succeeds,
        input: || over(&[(0, &[&[], &[]]), (0, &[&[]])], SetOp::Union),
        expect: Expect::Value(Published {
            rows: 1,
            cells: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("intersection_keeps_exact_shared_rows"),
        word: Word::Succeeds,
        input: || over(TWO_SETS, SetOp::Intersection),
        expect: Expect::Value(Published {
            rows: 1,
            cells: Cow::Borrowed(&[2]),
        }),
    },
    Case {
        name: CaseName("difference_keeps_exact_left_only_rows"),
        word: Word::Succeeds,
        input: || over(TWO_SETS, SetOp::Difference),
        expect: Expect::Value(Published {
            rows: 1,
            cells: Cow::Borrowed(&[1]),
        }),
    },
    Case {
        name: CaseName("union_refuses_unlike_columns"),
        word: Word::Refuses,
        input: || over(UNLIKE, SetOp::Union),
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("intersection_refuses_unlike_columns"),
        word: Word::Refuses,
        input: || over(UNLIKE, SetOp::Intersection),
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("difference_refuses_unlike_columns"),
        word: Word::Refuses,
        input: || over(UNLIKE, SetOp::Difference),
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("antijoin_keeps_exact_unmatched_left_row"),
        word: Word::Succeeds,
        input: || over(KEYED_BY_ONE, SetOp::Antijoin { left_on: 0 }),
        expect: Expect::Value(Published {
            rows: 1,
            cells: Cow::Borrowed(&[2, 20]),
        }),
    },
    Case {
        name: CaseName("antijoin_refuses_exact_missing_key_column"),
        word: Word::Refuses,
        input: || over(KEYED_BY_ONE, SetOp::Antijoin { left_on: 4 }),
        expect: Expect::Error(OperatingError::Table(RowsError::NoSuchColumn { index: 4 })),
    },
    Case {
        name: CaseName("a_key_the_right_holds_is_not_absent"),
        word: Word::Succeeds,
        input: || {
            over(
                &[(1, &[&[2]]), (1, &[&[1], &[2]])],
                SetOp::Antijoin { left_on: 0 },
            )
        },
        expect: Expect::Value(Published {
            rows: 0,
            cells: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("a_key_the_right_lacks_is_absent"),
        word: Word::Succeeds,
        input: || {
            over(
                &[(1, &[&[9]]), (1, &[&[1], &[2]])],
                SetOp::Antijoin { left_on: 0 },
            )
        },
        expect: Expect::Value(Published {
            rows: 1,
            cells: Cow::Borrowed(&[9]),
        }),
    },
    Case {
        name: CaseName("count_keeps_one_row_per_group_and_exact_sizes"),
        word: Word::Succeeds,
        input: || over(ONE_KEYED, SetOp::Count),
        expect: Expect::Value(Published {
            rows: 2,
            cells: Cow::Borrowed(&[1, 2, 2, 1]),
        }),
    },
    Case {
        name: CaseName("count_requires_its_real_provider"),
        word: Word::Refuses,
        input: || over(&[], SetOp::Count),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("changing_the_second_population_changes_exact_antijoin_output"),
        word: Word::Succeeds,
        input: || {
            over(
                &[(1, &[&[2]]), (1, &[&[1]])],
                SetOp::Antijoin { left_on: 0 },
            )
        },
        expect: Expect::Value(Published {
            rows: 1,
            cells: Cow::Borrowed(&[2]),
        }),
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
