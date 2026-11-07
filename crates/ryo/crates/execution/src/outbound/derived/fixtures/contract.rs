//! Behavioral contract shared by the injected derived adapter and test fake.
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_derived::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

type Input = ColumnIndex;
type Value = usize;
type Failure = DerivedFault;

/// Observable schema of a prepared equality request and the first absent column it refuses.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_request_on_a_present_column_keeps_its_schema"),
        word: Word::Succeeds,
        input: || ColumnIndex(0),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_request_on_an_absent_column_is_refused_on_validation"),
        word: Word::Refuses,
        input: || ColumnIndex(3),
        expect: Expect::Error(DerivedFault::NoSuchColumn {
            column: ColumnIndex(3),
        }),
    },
];

/// Run the same observable cases for every implementation of the engine port.
///
/// # Panics
///
/// When the implementation changes the schema it keeps or its refusal.
pub fn check(adapter: &impl DerivedPort) {
    let subject = |column: Input| {
        let prepared = adapter.prepare(
            DerivedRequest::Equality {
                columns: vec![column],
            },
            vec![vec![CellKind::Number]],
        );
        prepared.validate()?;
        Ok(prepared.schemas().len())
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
