//! The `preparation` service's cases, on values alone.

use super::service::{kind, prepare};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::{DerivedFault, InputSlot};
use crate::domain::request::types::{DerivedKind, DerivedRequest};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
enum Scenario {
    EqualityOnAPresentColumn,
    KindOfARequest,
    EqualityOnAnAbsentColumn,
    OrderedKeysWithoutASecondInput,
}
type Input = Scenario;
type Value = usize;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_request_on_a_present_column_prepares"),
        word: Word::Succeeds,
        input: || Scenario::EqualityOnAPresentColumn,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_kind_names_the_request_variant"),
        word: Word::Succeeds,
        input: || Scenario::KindOfARequest,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_request_on_an_absent_column_stores_the_fault"),
        word: Word::Refuses,
        input: || Scenario::EqualityOnAnAbsentColumn,
        expect: Expect::Error(DerivedFault::NoSuchColumn {
            column: ColumnIndex(3),
        }),
    },
    Case {
        name: CaseName("a_request_that_needs_two_inputs_refuses_with_one"),
        word: Word::Refuses,
        input: || Scenario::OrderedKeysWithoutASecondInput,
        expect: Expect::Error(DerivedFault::MissingInput {
            position: InputSlot(2),
        }),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let schemas = vec![vec![CellKind::Number]];
    let equality = |column| DerivedRequest::Equality {
        columns: vec![ColumnIndex(column)],
    };
    match input {
        Scenario::EqualityOnAPresentColumn => {
            let prepared = prepare(equality(0), schemas);
            prepared.validate()?;
            Ok(prepared.schemas().len())
        }
        Scenario::KindOfARequest => {
            Ok(kind(&equality(0)).ordinal()
                + usize::from(kind(&equality(0)) != DerivedKind::Equality))
        }
        Scenario::EqualityOnAnAbsentColumn => prepare(equality(3), schemas).validate().map(|()| 0),
        Scenario::OrderedKeysWithoutASecondInput => prepare(
            DerivedRequest::OrderedKeys {
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                left_order: ColumnIndex(0),
                right_order: ColumnIndex(0),
            },
            schemas,
        )
        .validate()
        .map(|()| 0),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
