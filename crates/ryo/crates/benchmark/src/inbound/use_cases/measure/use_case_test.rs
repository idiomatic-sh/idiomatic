//! Samples are released only when coverage is complete and results agree.
use super::service::measure;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::types::Audit;
use crate::domain::{diagnostics::errors::Error, suite::types::RunRequest};
use crate::outbound::{
    execution::fixtures::fake::ExecutionFake, linter::fixtures::fake::LinterFake,
};
type Input = bool;
type Value = bool;
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Incomplete,
    Unexpected,
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal reports release observations"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("incomplete coverage releases no observations"),
        word: Word::Refuses,
        input: || false,
        expect: Expect::Error(Failure::Incomplete),
    },
];
fn subject(complete: Input) -> Result<Value, Failure> {
    let audit = Audit {
        implemented: vec!["fixture".into()],
        missing: if complete {
            Vec::new()
        } else {
            vec!["missing".into()]
        },
    };
    let request = RunRequest {
        root: ".".into(),
        cache: "cache".into(),
        files: vec!["input.txt".into()],
    };
    measure(
        &audit,
        &LinterFake,
        &LinterFake,
        &ExecutionFake { status: 0 },
        &request,
    )
    .map(|pair| pair.reference.report == pair.candidate.report)
    .map_err(|error| match error {
        Error::Invalid(message)
            if message == "default coverage incomplete: 1 missing rules: missing" =>
        {
            Failure::Incomplete
        }
        _ => Failure::Unexpected,
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

type Failure = Refusal;
