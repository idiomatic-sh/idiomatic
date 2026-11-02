//! Exact diagnostic multisets and submitted-file sets determine agreement.
use super::{interface::CompareUseCase as _, use_case::CompareUseCaseImpl};
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
            reference_skipped: vec![],
            candidate_skipped: vec![],
            reference: Vec::new(),
            candidate: Vec::new(),
        }),
    },
];
fn matching() -> Input {
    let report = RunReport {
        execution_metadata: crate::domain::diagnostics::types::ExecutionMetadata::Complete,
        project_context: None,
        files: vec![ryo::linter::RelativePath("input.txt".into())],
        skipped: Vec::new(),
        coverage: Some(crate::domain::diagnostics::types::RuleCoverage {
            selected: 1,
            evaluated: 1,
            skipped: 0,
            unknown_or_unread: 0,
        }),
        diagnostics: Vec::new(),
    };
    (report.clone(), report)
}
fn missing_file() -> Input {
    let (reference, mut candidate) = matching();
    candidate.files.clear();
    (reference, candidate)
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "the shared case runner hands each independently constructed input to the subject by value"
)]
fn subject(input: Input) -> Result<Value, Failure> {
    CompareUseCaseImpl::new().compare(&input.0, &input.1)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
