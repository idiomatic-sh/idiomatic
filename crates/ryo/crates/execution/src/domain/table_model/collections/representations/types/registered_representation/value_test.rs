//! A foreign classification refuses before a malformed physical read can modify scratch.
use crate::domain::{
    operating::errors::OperatingError,
    table_model::collections::representations::{
        service::registered,
        types::{RepresentationInput, RepresentationKind, RowRead},
    },
};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::rows::types::{RowCount, RowIndex, Table},
};
type Input = RepresentationKind;
type Value = ();
type Failure = OperatingError;
fn subject(kind: Input) -> Result<Value, Failure> {
    let table = Table {
        kinds: vec![CellKind::Number],
        cells: Vec::new(),
        count: RowCount(1),
    };
    let mut decode = vec![Cell::Number(Integer(99))];
    let outcome = registered(kind).row(
        RepresentationInput::Computed(&table),
        RowIndex(0),
        &mut decode,
    );
    let answer = match outcome {
        Ok(RowRead::MissingPhysical) => Ok(()),
        Ok(RowRead::Borrowed(_) | RowRead::Decoded(_)) => {
            panic!("malformed physical input cannot supply a row")
        }
        Err(error) => Err(error),
    };
    assert_eq!(decode, [Cell::Number(Integer(99))]);
    answer
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_actual_computed_route_preserves_missing_physical_read"),
        word: Word::Succeeds,
        input: || RepresentationKind::Computed,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_foreign_route_refuses_before_decoding_malformed_rows"),
        word: Word::Refuses,
        input: || RepresentationKind::Stored,
        expect: Expect::Error(OperatingError::WrongOperation),
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
