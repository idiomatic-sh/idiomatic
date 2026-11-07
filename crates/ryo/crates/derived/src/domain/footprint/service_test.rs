//! The `footprint` service's cases, on values alone.

use super::service::{describe, working_bytes};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::BuildWorkspace;
use crate::domain::contract::types::derived_payload::value::{DerivedPayload, PayloadData};
use crate::domain::fault::types::DerivedFault;
use crate::domain::native::types::NativeFinding;
use crate::domain::positions::types::RowSlot;
use crate::domain::sharing::types::shared_positions::value::SharedPositions;
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
enum Scenario {
    PositionsAreOneSharedBacking,
    NativeHoldsNoSharedBacking,
    PositionBackingRecordsItsCapacity,
    EmptyWorkspaceHoldsNothing,
}
type Input = Scenario;
type Value = usize;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_positions_payload_is_one_shared_backing"),
        word: Word::Succeeds,
        input: || Scenario::PositionsAreOneSharedBacking,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_native_payload_holds_no_shared_backing"),
        word: Word::Succeeds,
        input: || Scenario::NativeHoldsNoSharedBacking,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("the_backing_records_the_capacity_of_its_positions"),
        word: Word::Succeeds,
        input: || Scenario::PositionBackingRecordsItsCapacity,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_empty_workspace_holds_no_bytes"),
        word: Word::Succeeds,
        input: || Scenario::EmptyWorkspaceHoldsNothing,
        expect: Expect::Value(0),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let positions = SharedPositions::new(vec![RowSlot(0), RowSlot(3)]);
    let payload = |data| DerivedPayload { data };
    match input {
        Scenario::PositionsAreOneSharedBacking => Ok(describe(&payload(
            PayloadData::PredicateMatches(positions),
        ))?
        .shared
        .len()),
        Scenario::NativeHoldsNoSharedBacking => Ok(describe(&payload(PayloadData::NativeLookup(
            NativeFinding::Span(ColumnIndex(0)),
        )))?
        .shared
        .len()),
        Scenario::PositionBackingRecordsItsCapacity => {
            let capacity = positions.capacity();
            let footprint = describe(&payload(PayloadData::PredicateMatches(positions)))?;
            Ok(usize::from(
                footprint
                    .shared
                    .iter()
                    .all(|backing| backing.position_capacity == capacity),
            ))
        }
        Scenario::EmptyWorkspaceHoldsNothing => Ok(working_bytes(&BuildWorkspace::default())?.0),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
