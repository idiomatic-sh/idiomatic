//! A registration lists every default; a missing implementation fails its row and an empty list is refused.
use super::{interface::VerifyInventoryUseCase as _, use_case::VerifyInventoryUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::types::inventory::value::InventoryVerdict;
use crate::outbound::{execution::fixtures::fake::ExecutionFake, inventory::factory::local};
use std::sync::Arc;

type Input = &'static [&'static str];
type Value = (InventoryVerdict, usize);
type Failure = usize;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a default without an implementation fails its row and launches nothing"),
        word: Word::Succeeds,
        input: || &["missing"],
        expect: Expect::Value((InventoryVerdict::Failed, 0)),
    },
    Case {
        name: CaseName("empty defaults are refused before any launch"),
        word: Word::Refuses,
        input: || &[],
        expect: Expect::Error(0),
    },
];

fn subject(defaults: Input) -> Result<Value, Failure> {
    let root = tempfile::tempdir().expect("inventory fixture");
    let registration = root.path().join("registration.json");
    let output = root.path().join("result");
    let rules = defaults
        .iter()
        .map(|rule| format!("\"{rule}\""))
        .collect::<Vec<_>>()
        .join(",");
    std::fs::write(
        &registration,
        format!(
            "{{\"authority\":{{\"kind\":\"unavailable\",\"reason\":\"fixture registration has no implementation\"}},\"provider\":\"fixture\",\"defaults\":[{rules}],\"rules\":[]}}"
        ),
    )
    .expect("registration fixture");
    let execution = Arc::new(ExecutionFake::default());
    let operation = VerifyInventoryUseCaseImpl::new(
        Arc::new(crate::outbound::suite::factory::local()),
        Arc::new(crate::outbound::receipts::factory::local(
            crate::outbound::receipts::types::CaseReceipts::Unrecorded,
        )),
        execution.clone(),
        Arc::new(local()),
    );
    let result = operation.verify_inventory(&registration, &output);
    let calls = execution.history.lock().expect("history").len();
    result
        .map(|verdict| (verdict, calls))
        .map_err(|_error| calls)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
