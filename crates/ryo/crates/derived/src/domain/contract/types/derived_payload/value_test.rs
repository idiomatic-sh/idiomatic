//! A payload lends only what its kind holds.
use super::value::{DerivedPayload, PayloadData};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::native::types::NativeFinding;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;

#[derive(Clone, Copy)]
enum Scenario {
    PartitionOrderPayloadReportsItsCapacity,
    NativePayloadReportsNoOrder,
}
type Input = Scenario;
type Value = usize;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_partition_order_payload_reports_the_capacity_of_its_order"),
        word: Word::Succeeds,
        input: || Scenario::PartitionOrderPayloadReportsItsCapacity,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_native_payload_reports_no_order"),
        word: Word::Succeeds,
        input: || Scenario::NativePayloadReportsNoOrder,
        expect: Expect::Value(0),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let payload = DerivedPayload {
        data: match input {
            Scenario::PartitionOrderPayloadReportsItsCapacity => {
                PayloadData::PartitionOrder(vec![RowIndex(0), RowIndex(4)])
            }
            Scenario::NativePayloadReportsNoOrder => {
                PayloadData::NativeLookup(NativeFinding::Span(ColumnIndex(0)))
            }
        },
    };
    Ok(payload
        .partition_order_capacity()
        .map_or(0, |capacity| capacity.0))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
