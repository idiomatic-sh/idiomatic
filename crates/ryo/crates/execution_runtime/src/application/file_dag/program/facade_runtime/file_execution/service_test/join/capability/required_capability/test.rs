//! Every provider kind reaches the required join capability, and empty or failed lefts never build a right index.
use super::super::super::fixtures::execution::{NOTHING_HELD, SourceAttempt};
use super::super::fixtures::REFUSAL;
use super::fixtures::{Capability, Delivered, Gate, Join, Observed, Supply, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::rows::errors::RowsError;
use std::borrow::Cow;

type Input = Join;
type Value = Observed;
type Failure = OperatingError;

/// A witness-refused run that delivers nothing, asks the witness once, and whose real rerun
/// delivers `accepted` rows; both leave nothing held.
const fn refused(delivery: bool, optional_limit: Option<usize>, accepted: usize) -> Capability {
    Capability {
        delivery,
        optional_limit,
        refused: Delivered::Error(OperatingError::Table(RowsError::NoSuchColumn {
            index: REFUSAL,
        })),
        prefix_rows: 0,
        witness_calls: 1,
        accepted: Delivered::Rows(accepted),
        held: [NOTHING_HELD; 2],
    }
}

/// Every delivery and optional limit refused by the witness, each rerun delivering `accepted` rows.
const fn every_run(accepted: usize) -> [Capability; 4] {
    [
        refused(false, None, accepted),
        refused(false, Some(0), accepted),
        refused(true, None, accepted),
        refused(true, Some(0), accepted),
    ]
}

const NUMBER_RUNS: [Capability; 4] = every_run(2);
const TREE_RUNS: [Capability; 4] = every_run(1);

/// An empty left that delivers no rows, admits both sources, builds no index and leaves nothing held.
const EMPTY: Gate = Gate {
    delivered: Delivered::Rows(0),
    witness_calls: 0,
    attempts: Cow::Borrowed(&[SourceAttempt::Available(0), SourceAttempt::Available(1)]),
    reads: [1, 1],
    index: (0, 0),
    prefix_rows: 0,
    held: NOTHING_HELD,
};

/// A left that fails before the join: its error, one refused attempt and one read, and nothing held.
const FAILED: Gate = Gate {
    delivered: Delivered::Error(OperatingError::NotRun),
    witness_calls: 0,
    attempts: Cow::Borrowed(&[SourceAttempt::Refused(0, OperatingError::NotRun)]),
    reads: [1, 0],
    index: (0, 0),
    prefix_rows: 0,
    held: NOTHING_HELD,
};

const fn empty_left(left: Option<OperatingError>, delivery: bool) -> Join {
    Join::EmptyLeft {
        left,
        right: None,
        delivery,
        refusing: true,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "computed_stored_compact_and_tree_providers_reach_the_same_required_capability",
        ),
        word: Word::Succeeds,
        input: || Join::Number {
            right: Supply::Computed,
        },
        expect: Expect::Value(Observed::Capability {
            runs: Cow::Borrowed(&NUMBER_RUNS),
            admitted_names: 0,
        }),
    },
    Case {
        name: CaseName("a_stored_right_reaches_the_required_capability"),
        word: Word::Succeeds,
        input: || Join::Number {
            right: Supply::Stored,
        },
        expect: Expect::Value(Observed::Capability {
            runs: Cow::Borrowed(&NUMBER_RUNS),
            admitted_names: 0,
        }),
    },
    Case {
        name: CaseName("a_compact_right_reaches_the_required_capability"),
        word: Word::Succeeds,
        input: || Join::Number {
            right: Supply::Compact,
        },
        expect: Expect::Value(Observed::Capability {
            runs: Cow::Borrowed(&NUMBER_RUNS),
            admitted_names: 0,
        }),
    },
    Case {
        name: CaseName("a_tree_right_reaches_the_required_capability"),
        word: Word::Succeeds,
        input: || Join::Tree { tree_left: false },
        expect: Expect::Value(Observed::Capability {
            runs: Cow::Borrowed(&TREE_RUNS),
            admitted_names: 0,
        }),
    },
    Case {
        name: CaseName("a_tree_left_reaches_the_required_capability"),
        word: Word::Succeeds,
        input: || Join::Tree { tree_left: true },
        expect: Expect::Value(Observed::Capability {
            runs: Cow::Borrowed(&TREE_RUNS),
            admitted_names: 0,
        }),
    },
    Case {
        name: CaseName(
            "empty_or_failed_left_never_enters_the_join_capability_or_builds_a_rhs_index",
        ),
        word: Word::Succeeds,
        input: || empty_left(None, false),
        expect: Expect::Value(Observed::Gate(EMPTY)),
    },
    Case {
        name: CaseName("a_delivered_empty_left_never_enters_the_join_capability"),
        word: Word::Succeeds,
        input: || empty_left(None, true),
        expect: Expect::Value(Observed::Gate(EMPTY)),
    },
    Case {
        name: CaseName("a_failed_left_never_enters_the_join_capability"),
        word: Word::Succeeds,
        input: || empty_left(Some(OperatingError::NotRun), false),
        expect: Expect::Value(Observed::Gate(FAILED)),
    },
    Case {
        name: CaseName("a_delivered_failed_left_never_enters_the_join_capability"),
        word: Word::Succeeds,
        input: || empty_left(Some(OperatingError::NotRun), true),
        expect: Expect::Value(Observed::Gate(FAILED)),
    },
    Case {
        name: CaseName("a_failed_right_under_the_real_operators_builds_no_index"),
        word: Word::Succeeds,
        input: || Join::EmptyLeft {
            left: None,
            right: Some(OperatingError::NotRead),
            delivery: true,
            refusing: false,
        },
        expect: Expect::Value(Observed::Gate(Gate {
            delivered: Delivered::Error(OperatingError::NotRead),
            witness_calls: 0,
            attempts: Cow::Borrowed(&[
                SourceAttempt::Available(0),
                SourceAttempt::Refused(1, OperatingError::NotRead),
                SourceAttempt::Available(0),
                SourceAttempt::Available(0),
                SourceAttempt::Available(0),
            ]),
            reads: [4, 1],
            index: (0, 0),
            prefix_rows: 0,
            held: NOTHING_HELD,
        })),
    },
];

/// Runs the join and observes it.
fn subject(join: Input) -> Result<Value, Failure> {
    exercise(join)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
