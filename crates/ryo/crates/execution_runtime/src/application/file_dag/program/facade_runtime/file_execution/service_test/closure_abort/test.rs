//! A real downstream refusal interrupts partition construction and releases its backing.
use super::fixtures::{Delivered, Observed, Refusal, Refused, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use std::borrow::Cow;

type Input = (Refusal, bool);
type Value = Observed;
type Failure = String;

/// A refusal by a bad pattern on the second partition, after `builds` partition builds.
const fn pattern(fused: bool, builds: usize) -> Observed {
    Observed {
        fused,
        refused: Some(Refused::InvalidRegex),
        rows_delivered: 0,
        first_row: None,
        partition_builds: builds,
        order_first_on_keys: true,
        requests: 1 + builds,
        rest_are_adjacency: true,
        backing_released: true,
        working_bytes: 0,
        publication_bytes: 0,
    }
}

/// A sink refusing the first delivered row, after three partition builds.
const fn callback(fused: bool) -> Observed {
    Observed {
        fused,
        refused: Some(Refused::Error(OperatingError::WrongBinding)),
        rows_delivered: 1,
        first_row: Some(Delivered {
            key: Cell::Number(Integer(0)),
            text: Cow::Borrowed("a"),
            endpoints: [Cell::Number(Integer(1)), Cell::Number(Integer(2))],
        }),
        partition_builds: 3,
        order_first_on_keys: true,
        requests: 4,
        rest_are_adjacency: true,
        backing_released: true,
        working_bytes: 0,
        publication_bytes: 0,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("later_partition_refusal_stops_work_and_releases_backing"),
        word: Word::Succeeds,
        input: || (Refusal::CollectedPattern, true),
        expect: Expect::Value(pattern(true, 2)),
    },
    Case {
        name: CaseName("a_collected_pattern_refusal_materialized_builds_every_partition"),
        word: Word::Succeeds,
        input: || (Refusal::CollectedPattern, false),
        expect: Expect::Value(pattern(false, 3)),
    },
    Case {
        name: CaseName("a_delivered_pattern_refusal_fused_stops_at_the_failing_partition"),
        word: Word::Succeeds,
        input: || (Refusal::DeliveredPattern, true),
        expect: Expect::Value(pattern(true, 2)),
    },
    Case {
        name: CaseName("a_delivered_pattern_refusal_materialized_builds_every_partition"),
        word: Word::Succeeds,
        input: || (Refusal::DeliveredPattern, false),
        expect: Expect::Value(pattern(false, 3)),
    },
    Case {
        name: CaseName("a_sink_refusal_fused_stops_delivery_at_the_first_row"),
        word: Word::Succeeds,
        input: || (Refusal::Callback, true),
        expect: Expect::Value(callback(true)),
    },
    Case {
        name: CaseName("a_sink_refusal_materialized_stops_delivery_at_the_first_row"),
        word: Word::Succeeds,
        input: || (Refusal::Callback, false),
        expect: Expect::Value(callback(false)),
    },
];

/// Runs the closure with the refusal, fused or materialized.
fn subject((refusal, fused): Input) -> Result<Value, Failure> {
    Ok(exercise(refusal, fused))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
