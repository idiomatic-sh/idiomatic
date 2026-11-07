//! Exact diagnostic multisets and submitted-file sets determine agreement.
use super::{interface::CompareUseCase as _, use_case::CompareUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::reports::types::Difference;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
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
        execution_metadata:
            ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
        project_context: ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
        files: vec![ryo::linter::RelativePath("input.txt".into())],
        skipped: Vec::new(),
        coverage: ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
            ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                selected: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
            },
        ),
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
