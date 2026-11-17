//! The antijoin and the join through `plan` and `run` only, the meanings main's row-mode cases stated: which left rows an
//! antijoin keeps, when it reads its left key, and a join's order, view mapping and the columns it reads; and that a join of
//! filtered inputs answers what a filter of the join answers, row for row and in order, which the planner's placement of
//! filters relies on.
use super::super::super::fixtures::cases::{compared, input_of, named, select};
use super::super::super::fixtures::relational::{answered, both, join, node, project, relation};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, LiteralText, Operand, PlanComparison, QueryLiteral, RepeatedRows, Side, Test,
};
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
    ProbeOfFilteredSides,
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
    // A join's rows are a typed table, and a right column past the right rows has no kind: the join is refused before any
    // row, whatever it matches, as the schema of the step refuses it before a row is read.
    Case {
        name: CaseName(
            "a_join_naming_a_column_past_its_right_rows_is_refused_whatever_its_matches",
        ),
        word: Word::Refuses,
        input: || Scenario::JoinOfNoMatchPastItsOutput,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(99),
        }),
    },
    Case {
        name: CaseName("missing_left_key_is_refused_before_candidate_access"),
        word: Word::Refuses,
        input: || Scenario::JoinPastItsLeftKey,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(9),
        }),
    },
    // Names repeat on both sides: each left row of refinement r1 meets every right row of refinement r2 with its name, in the
    // left's order and then the right's.
    Case {
        name: CaseName("a_probe_of_filtered_sides_answers_the_filtered_probe"),
        word: Word::Succeeds,
        input: || Scenario::ProbeOfFilteredSides,
        expect: Expect::Value(Cow::Borrowed(
            "n0 n2 a r1 r2 | n0 n4 a r1 r2 | n3 n1 b r1 r2 | n3 n6 b r1 r2 | n5 n2 a r1 r2 | n5 n4 a r1 r2",
        )),
    },
];

/// Column `column` equals the text `text`.
fn equal(column: u32, text: &str) -> PlanComparison {
    compared(
        Test::Equal,
        Operand::Column(ColumnIndex(column)),
        Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
    )
}

/// The rows a join of names answers when a filter of its answer keeps the left's refinement r1 and the right's r2, and the
/// rows it answers over a filter of each side by the same; when the two differ, both, apart.
fn probe_of_filtered_sides() -> Result<String, Failure> {
    let pairs = [
        ("r1", "a"),
        ("r2", "b"),
        ("r2", "a"),
        ("r1", "b"),
        ("r2", "a"),
        ("r1", "a"),
        ("r2", "b"),
    ];
    let output = [
        (Side::Left, 0),
        (Side::Right, 0),
        (Side::Left, 3),
        (Side::Left, 2),
        (Side::Right, 2),
    ];
    let filtered_probe = answered(
        &[
            join((relation(), 3), (relation(), 3), &output),
            select(node(0), vec![equal(3, "r1"), equal(4, "r2")]),
        ],
        &[1],
        &pairs,
    )?;
    let probe_of_filtered = answered(
        &[
            select(relation(), vec![equal(2, "r1")]),
            select(relation(), vec![equal(2, "r2")]),
            join((node(0), 3), (node(1), 3), &output),
        ],
        &[2],
        &pairs,
    )?;
    Ok(if filtered_probe == probe_of_filtered {
        filtered_probe
    } else {
        format!("{filtered_probe} differs from {probe_of_filtered}")
    })
}

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
        Scenario::ProbeOfFilteredSides => probe_of_filtered_sides()?,
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
