//! The derived fake preserves the port contract and records each call once.
use super::{super::contract, DerivedFake};
use crate::outbound::derived::interface::DerivedPort as _;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::atomic::Ordering;

/// What the fake answered for a prepared request: its schemas, and the calls it recorded.
#[derive(Debug, PartialEq)]
struct Prepared {
    schemas: usize,
    calls: usize,
}

/// The fault a prepared request's validation raised, and the calls the fake recorded.
#[derive(Debug, PartialEq)]
struct Refused {
    fault: DerivedFault,
    calls: usize,
}

type Input = ColumnIndex;
type Value = Prepared;
type Failure = Refused;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("fake_prepares_a_present_column_and_records_once"),
        word: Word::Succeeds,
        input: || ColumnIndex(0),
        expect: Expect::Value(Prepared {
            schemas: 1,
            calls: 1,
        }),
    },
    Case {
        name: CaseName("fake_refuses_an_absent_column_and_records_once"),
        word: Word::Refuses,
        input: || ColumnIndex(3),
        expect: Expect::Error(Refused {
            fault: DerivedFault::NoSuchColumn {
                column: ColumnIndex(3),
            },
            calls: 1,
        }),
    },
];

fn subject(column: Input) -> Result<Value, Failure> {
    let fake = DerivedFake::default();
    let prepared = fake.prepare(
        DerivedRequest::Equality {
            columns: vec![column],
        },
        vec![vec![CellKind::Number]].into(),
    );
    let calls = fake.calls.load(Ordering::Relaxed);
    prepared
        .validate()
        .map(|()| Prepared {
            schemas: prepared.schemas().len(),
            calls,
        })
        .map_err(|fault| Refused { fault, calls })
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
