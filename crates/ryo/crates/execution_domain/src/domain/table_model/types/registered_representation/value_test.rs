//! Missing rows and malformed rows stay distinct; foreign classifications refuse before reading.
use crate::domain::{
    operating::errors::OperatingError,
    table_model::{
        collections::representations::types::RowRead, service::registered,
        types::RepresentationInput, types::RepresentationKind,
    },
};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::rows::types::{RowCount, RowIndex, Table},
};
type Input = (RepresentationKind, Vec<Cell>, RowIndex);
type Value = ();
type Failure = OperatingError;
fn subject((kind, cells, row): Input) -> Result<Value, Failure> {
    let table = Table {
        kinds: vec![CellKind::Number],
        cells,
        count: RowCount(1),
    };
    let mut decode = vec![Cell::Number(Integer(99))];
    let outcome = registered(kind).row(RepresentationInput::Computed(&table), row, &mut decode);
    let answer = match outcome {
        Ok(RowRead::MissingPhysical) => Ok(()),
        Ok(RowRead::Borrowed(_) | RowRead::Decoded(_)) => {
            panic!("a missing or malformed physical row cannot supply a row")
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
        input: || {
            (
                RepresentationKind::Computed,
                vec![Cell::Number(Integer(7))],
                RowIndex(1),
            )
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_foreign_route_refuses_before_decoding_malformed_rows"),
        word: Word::Refuses,
        input: || (RepresentationKind::Stored, Vec::new(), RowIndex(0)),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_actual_computed_route_refuses_a_malformed_in_range_row"),
        word: Word::Refuses,
        input: || (RepresentationKind::Computed, Vec::new(), RowIndex(0)),
        expect: Expect::Error(OperatingError::ShapeMismatch),
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
