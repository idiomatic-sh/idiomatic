//! Delivery refusal and a semantic refusal release every strategy; the semantic error is primary.
//! `execute` finishes the owner and asserts the shared ledger, working and owned bytes are zero.
use super::super::fixtures::execution::NOTHING_HELD;
use super::fixtures::{Delivered, Refused, Run, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::rows::errors::RowsError;

type Input = Run;
type Value = Delivered;
type Failure = Refused;

/// A run of the strategy with this meaning and sink.
const fn strategy(tree: bool, meaning_refuses: bool, sink_refuses: bool) -> Run {
    Run {
        tree,
        meaning_refuses,
        sink_refuses,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "native_tree_join_delivery_refusal_releases_and_the_completed_join_delivers",
        ),
        word: Word::Refuses,
        input: || strategy(true, false, true),
        expect: Expect::Error(Refused {
            meaning_ran: true,
            error: OperatingError::NotRun,
            delivered: 0,
            held: NOTHING_HELD,
        }),
    },
    Case {
        name: CaseName("the_completed_native_tree_join_delivers"),
        word: Word::Succeeds,
        input: || strategy(true, false, false),
        expect: Expect::Value(Delivered {
            meaning_ran: true,
            rows: 2,
            delivered: 2,
            held: NOTHING_HELD,
        }),
    },
    Case {
        name: CaseName("native_tree_join_semantic_refusal_is_primary_over_the_sink_refusal"),
        word: Word::Refuses,
        input: || strategy(true, true, true),
        expect: Expect::Error(Refused {
            meaning_ran: true,
            error: OperatingError::Table(RowsError::NoSuchColumn { index: 60001 }),
            delivered: 0,
            held: NOTHING_HELD,
        }),
    },
    Case {
        name: CaseName(
            "ordered_strategy_delivery_refusal_releases_and_the_completed_root_delivers",
        ),
        word: Word::Refuses,
        input: || strategy(false, false, true),
        expect: Expect::Error(Refused {
            meaning_ran: true,
            error: OperatingError::NotRun,
            delivered: 0,
            held: NOTHING_HELD,
        }),
    },
    Case {
        name: CaseName("the_completed_ordered_root_delivers"),
        word: Word::Succeeds,
        input: || strategy(false, false, false),
        expect: Expect::Value(Delivered {
            meaning_ran: true,
            rows: 1,
            delivered: 1,
            held: NOTHING_HELD,
        }),
    },
    Case {
        name: CaseName("ordered_strategy_semantic_refusal_is_primary_over_the_sink_refusal"),
        word: Word::Refuses,
        input: || strategy(false, true, true),
        expect: Expect::Error(Refused {
            meaning_ran: true,
            error: OperatingError::Table(RowsError::NoSuchColumn { index: 60001 }),
            delivered: 0,
            held: NOTHING_HELD,
        }),
    },
];

/// Runs the strategy with its meaning and sink.
fn subject(run: Input) -> Result<Value, Failure> {
    exercise(run)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
