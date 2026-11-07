//! A payload shares only what its kind holds.
use super::value::{DerivedPayload, PayloadData};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::native::types::NativeFinding;
use crate::domain::positions::types::RowPosition;
use crate::domain::sharing::types::shared_positions::value::SharedPositions;
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
enum Scenario {
    PredicatePayloadSharesPositions,
    NativePayloadSharesNothing,
}
type Input = Scenario;
type Value = usize;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_predicate_payload_shares_its_positions"),
        word: Word::Succeeds,
        input: || Scenario::PredicatePayloadSharesPositions,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_native_payload_shares_no_positions"),
        word: Word::Succeeds,
        input: || Scenario::NativePayloadSharesNothing,
        expect: Expect::Value(0),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let payload = DerivedPayload {
        data: match input {
            Scenario::PredicatePayloadSharesPositions => PayloadData::PredicateMatches(
                SharedPositions::new(vec![RowPosition(0), RowPosition(4)]),
            ),
            Scenario::NativePayloadSharesNothing => {
                PayloadData::NativeLookup(NativeFinding::Span(ColumnIndex(0)))
            }
        },
    };
    Ok(payload
        .share_positions()
        .map_or(0, |rows| rows.as_slice().len()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
