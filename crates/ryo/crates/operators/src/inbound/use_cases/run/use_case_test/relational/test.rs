//! The antijoin and the join through `plan` and `run` only, the meanings main's row-mode cases stated: which left rows an
//! antijoin keeps, when it reads its left key, and a join's order, view mapping and the columns it reads.
use super::super::super::fixtures::cases::{input_of, named, select};
use super::super::super::fixtures::relational::{answered, both, join, node, project, relation};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use ryo_values::domain::plan_parts::types::{ColumnIndex, RepeatedRows, Side};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    AntijoinOfPresentAndAbsentKeys,
    AntijoinOfNoRightRowsPastItsKey,
    AntijoinOfRightRowsPastItsKey,
    JoinThroughAView,
    JoinOfNoMatchPastItsOutput,
    JoinPastItsLeftKey,
}
type Input = Scenario;
/// The rows the emitted node answers, each cell a node's place, a text or a number.
type Value = Cow<'static, str>;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("antijoin_retains_duplicate_when_rhs_absent_and_skips_present_key"),
        word: Word::Succeeds,
        input: || Scenario::AntijoinOfPresentAndAbsentKeys,
        // Names a, b, a against the right's b: both a rows stay, the b row goes.
        expect: Expect::Value(Cow::Borrowed("a | a")),
    },
    Case {
        name: CaseName("empty_right_antijoin_never_reads_invalid_left_key"),
        word: Word::Succeeds,
        input: || Scenario::AntijoinOfNoRightRowsPastItsKey,
        expect: Expect::Value(Cow::Borrowed("a | b | a")),
    },
    Case {
        name: CaseName("nonempty_right_antijoin_checks_first_missing_key"),
        word: Word::Refuses,
        input: || Scenario::AntijoinOfRightRowsPastItsKey,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(5),
        }),
    },
    Case {
        name: CaseName("joined_rows_preserve_bag_order_duplicate_columns_and_view_mapping"),
        word: Word::Succeeds,
        input: || Scenario::JoinThroughAView,
        // The right read through the view (name, node): its column 0 is the name and 1 the node, repeated in the answer.
        expect: Expect::Value(Cow::Borrowed(
            "n0 n0 b b | n0 n2 b b | n1 n1 a a | n2 n0 b b | n2 n2 b b",
        )),
    },
    Case {
        name: CaseName("empty_matches_do_not_access_invalid_output_columns"),
        word: Word::Succeeds,
        input: || Scenario::JoinOfNoMatchPastItsOutput,
        expect: Expect::Value(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("missing_left_key_is_refused_before_candidate_access"),
        word: Word::Refuses,
        input: || Scenario::JoinPastItsLeftKey,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(9),
        }),
    },
];

/// The antijoin of node 0's one column of names with the rows of node 1 on their name, the left keyed on `left_key`.
fn antijoin_of_names(left_key: u32, right_name: &str) -> Result<String, Failure> {
    answered(
        &[
            project(relation(), &[3], RepeatedRows::Kept),
            select(relation(), vec![named(right_name)]),
            both(
                Operation::Antijoin {
                    left: input_of(node(0)),
                    right: input_of(node(1)),
                    left_on: vec![ColumnIndex(left_key)],
                    right_on: vec![ColumnIndex(3)],
                },
                node(0),
                node(1),
            ),
        ],
        &[2],
        &[("r1", "a"), ("r1", "b"), ("r1", "a")],
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let repeated = [("r1", "a"), ("r1", "b"), ("r1", "a")];
    Ok(Cow::Owned(match scenario {
        Scenario::AntijoinOfPresentAndAbsentKeys => antijoin_of_names(0, "b")?,
        Scenario::AntijoinOfNoRightRowsPastItsKey => antijoin_of_names(5, "zzz")?,
        Scenario::AntijoinOfRightRowsPastItsKey => antijoin_of_names(5, "b")?,
        Scenario::JoinThroughAView => {
            let mut through = join(
                (relation(), 3),
                (relation(), 0),
                &[
                    (Side::Left, 0),
                    (Side::Right, 1),
                    (Side::Right, 0),
                    (Side::Left, 3),
                ],
            );
            if let Operation::Join { view, .. } = &mut through.operation {
                *view = vec![ColumnIndex(3), ColumnIndex(0)];
            }
            answered(&[through], &[0], &[("r1", "b"), ("r1", "a"), ("r1", "b")])?
        }
        Scenario::JoinOfNoMatchPastItsOutput => answered(
            &[
                select(relation(), vec![named("zzz")]),
                join(
                    (relation(), 3),
                    (node(0), 3),
                    &[(Side::Left, 0), (Side::Right, 99)],
                ),
            ],
            &[1],
            &repeated,
        )?,
        Scenario::JoinPastItsLeftKey => answered(
            &[join((relation(), 9), (relation(), 3), &[(Side::Left, 0)])],
            &[0],
            &repeated,
        )?,
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
