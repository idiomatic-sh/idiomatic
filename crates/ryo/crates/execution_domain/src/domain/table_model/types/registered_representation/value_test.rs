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
use std::borrow::Cow;

/// Which row a representation answers, by its form; the borrowed and decoded cells are not compared.
#[derive(Debug, PartialEq)]
enum Answered {
    MissingPhysical,
    Borrowed,
    Decoded,
}

/// What the read answered, and the decode buffer after it (it held one cell, 99).
#[derive(Debug, PartialEq)]
struct Observed {
    read: Answered,
    decode: Cow<'static, [Cell]>,
}

/// The refusal, and the decode buffer after it.
#[derive(Debug, PartialEq)]
struct Refused {
    error: OperatingError,
    decode: Cow<'static, [Cell]>,
}

/// The decode buffer as the read found it.
const UNTOUCHED: &[Cell] = &[Cell::Number(Integer(99))];

type Input = (RepresentationKind, Vec<Cell>, RowIndex);
type Value = Observed;
type Failure = Refused;
fn subject((kind, cells, row): Input) -> Result<Value, Failure> {
    let table = Table {
        kinds: vec![CellKind::Number],
        cells,
        count: RowCount(1),
    };
    let mut decode = vec![Cell::Number(Integer(99))];
    let outcome = registered(kind)
        .row(RepresentationInput::Computed(&table), row, &mut decode)
        .map(|read| match read {
            RowRead::MissingPhysical => Answered::MissingPhysical,
            RowRead::Borrowed(_) => Answered::Borrowed,
            RowRead::Decoded(_) => Answered::Decoded,
        });
    let decode: Cow<'static, [Cell]> = Cow::Owned(decode);
    outcome
        .map(|read| Observed {
            read,
            decode: decode.clone(),
        })
        .map_err(|error| Refused { error, decode })
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
        expect: Expect::Value(Observed {
            read: Answered::MissingPhysical,
            decode: Cow::Borrowed(UNTOUCHED),
        }),
    },
    Case {
        name: CaseName("a_foreign_route_refuses_before_decoding_malformed_rows"),
        word: Word::Refuses,
        input: || (RepresentationKind::Stored, Vec::new(), RowIndex(0)),
        expect: Expect::Error(Refused {
            error: OperatingError::WrongOperation,
            decode: Cow::Borrowed(UNTOUCHED),
        }),
    },
    Case {
        name: CaseName("the_actual_computed_route_refuses_a_malformed_in_range_row"),
        word: Word::Refuses,
        input: || (RepresentationKind::Computed, Vec::new(), RowIndex(0)),
        expect: Expect::Error(Refused {
            error: OperatingError::ShapeMismatch,
            decode: Cow::Borrowed(UNTOUCHED),
        }),
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
