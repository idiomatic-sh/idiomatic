//! The actual certification/measurement pipeline supplies every publication sample.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::certify::{
    fixtures::{Change, exercise},
    use_case::CertifyUseCaseImpl,
};
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;

type Input = Change;
type Value = CaseCount;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a full inventory publishes through the real public operation"),
        word: Word::Succeeds,
        input: || Change::InventoryFull,
        expect: Expect::Value(CaseCount(2)),
    },
    Case {
        name: CaseName("an unchanged diagnostic pair publishes through the real public operation"),
        word: Word::Succeeds,
        input: || Change::None,
        expect: Expect::Value(CaseCount(2)),
    },
];

fn subject(change: Input) -> Result<Value, Failure> {
    exercise(change, CertifyUseCaseImpl::new)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
