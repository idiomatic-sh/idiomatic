//! The local inventory adapter runs the core's contract rows, then the cases specific to it.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::interface::InventoryPort;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;

/// What a row gives the adapter: the bytes of an invocation record.
type Input = &'static str;
/// Whether the adapter decoded the record.
type Value = bool;
/// The adapter refused the record.
type Failure = ();

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a complete invocation record is decoded"),
        word: Word::Succeeds,
        input: || {
            r#"{"actual_request_root":"/root","files":["case.py"],"arguments":["--check"],"environment":{},"directory":"/root","exit":2,"signal":null,"observer_no_deadline":true,"metadata_error":"identity unavailable"}"#
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a record with a surprise field is refused"),
        word: Word::Refuses,
        input: || r#"{"surprise":1}"#,
        expect: Expect::Error(()),
    },
];

/// Decode one record through the factory's adapter, for the contract's rows and for its own.
fn subject(record: Input) -> Result<Value, Failure> {
    crate::outbound::inventory::factory::local()
        .decode_invocation(&CapturedBytes(record.as_bytes().to_vec()))
        .map(|_| true)
        .map_err(|_refused| ())
}

#[test]
fn succeeds() {
    crate::outbound::inventory::fixtures::contract::check(
        &crate::outbound::inventory::factory::local(),
    );
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
