//! The derived fake preserves the port contract and records each call once.
use super::{super::contract, DerivedFake};
use crate::outbound::derived::interface::DerivedPort as _;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::atomic::Ordering;

type Input = ColumnIndex;
type Value = usize;
type Failure = DerivedFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("fake_prepares_a_present_column_and_records_once"),
        word: Word::Succeeds,
        input: || ColumnIndex(0),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("fake_refuses_an_absent_column_and_records_once"),
        word: Word::Refuses,
        input: || ColumnIndex(3),
        expect: Expect::Error(DerivedFault::NoSuchColumn {
            column: ColumnIndex(3),
        }),
    },
];

fn subject(column: Input) -> Result<Value, Failure> {
    let fake = DerivedFake::default();
    let prepared = fake.prepare(
        DerivedRequest::Equality {
            columns: vec![column],
        },
        vec![vec![CellKind::Number]],
    );
    assert_eq!(fake.calls.load(Ordering::Relaxed), 1);
    prepared.validate()?;
    Ok(prepared.schemas().len())
}
#[test]
fn succeeds() {
    contract::check(&DerivedFake::default());
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
