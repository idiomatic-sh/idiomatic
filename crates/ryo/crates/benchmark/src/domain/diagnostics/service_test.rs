//! Exact diagnostic multisets and submitted-file sets determine agreement.
use super::service::compare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::types::RunReport;
type Input = (RunReport, RunReport);
type Value = ();
type Failure = ();
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
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("duplicate findings remain part of the multiset"),
        word: Word::Refuses,
        input: duplicate,
        expect: Expect::Error(()),
    },
];
fn duplicate() -> Input {
    let (mut reference, mut candidate) = matching();
    let finding = crate::domain::diagnostics::types::BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from("A").expect("fixture rule"),
        file: ryo::linter::RelativePath("input.txt".into()),
        start: crate::domain::diagnostics::types::SourceByteOffset(0),
        end: crate::domain::diagnostics::types::SourceByteOffset(1),
    };
    reference.diagnostics = vec![finding.clone()];
    candidate.diagnostics = vec![finding.clone(), finding];
    (reference, candidate)
}
fn matching() -> Input {
    let report = RunReport {
        project_context: None,
        files: vec![ryo::linter::RelativePath("input.txt".into())],
        skipped: Vec::new(),
        coverage: None,
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
    compare(&input.0, &input.1).map_err(|_error| ())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
