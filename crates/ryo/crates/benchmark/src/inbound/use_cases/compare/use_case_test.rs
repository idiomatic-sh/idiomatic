//! Exact diagnostic multisets and submitted-file sets determine agreement.
use super::service::compare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::types::Difference;
use crate::domain::diagnostics::types::RunReport;
type Input = (RunReport, RunReport);
type Value = ();
type Failure = Difference;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal empty findings and checked input"),
        word: Word::Succeeds,
        input: matching,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("different checked files are not agreement"),
        word: Word::Refuses,
        input: missing_file,
        expect: Expect::Error(Difference {
            reference: Vec::new(),
            candidate: Vec::new(),
        }),
    },
];
fn matching() -> Input {
    let report = RunReport {
        files: vec!["input.txt".into()],
        diagnostics: Vec::new(),
    };
    (report.clone(), report)
}
fn missing_file() -> Input {
    let (reference, mut candidate) = matching();
    candidate.files.clear();
    (reference, candidate)
}
fn subject(input: Input) -> Result<Value, Failure> {
    compare(&input.0, &input.1)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
