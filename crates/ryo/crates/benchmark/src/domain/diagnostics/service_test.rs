//! Exact diagnostic multisets and submitted-file sets determine agreement.
use super::service::compare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::reports::types::RunReport;
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
    let finding = ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from("A").expect("fixture rule"),
        file: ryo::linter::RelativePath("input.txt".into()),
        start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(0),
        end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(1),
    };
    reference.diagnostics = vec![finding.clone()];
    candidate.diagnostics = vec![finding.clone(), finding];
    (reference, candidate)
}
fn matching() -> Input {
    let report = RunReport {
        execution_metadata:
            ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
        project_context: None,
        files: vec![ryo::linter::RelativePath("input.txt".into())],
        skipped: Vec::new(),
        coverage: Some(
            ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                selected: 1,
                evaluated: 1,
                skipped: 0,
                unknown_or_unread: 0,
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

#[test]
fn unavailable_or_missing_reference_metadata_is_incomparable_not_known_zero() {
    use super::service::{compare_dimensions, compare_with_contract};
    use ryo_benchmark_contracts::domain::reports::types::Comparison;
    use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
    use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
    for capability in [
        ExecutionMetadata::Unknown,
        ExecutionMetadata::Unavailable {
            reason: "findings-only format".into(),
        },
    ] {
        let (mut reference, candidate) = matching();
        reference.execution_metadata = capability;
        reference.coverage = None;
        let dimensions =
            compare_with_contract(&reference, &candidate, ComparisonContract::Diagnostics)
                .expect("diagnostic timing contract may retain unobservable reference execution");
        assert_eq!(dimensions.diagnostics, Comparison::Exact);
        assert_eq!(dimensions.execution_metadata, Comparison::Incomparable);
        assert_eq!(compare_dimensions(&reference, &candidate), dimensions);
        assert!(
            compare(&reference, &candidate).is_err(),
            "full parity remains unproven"
        );
    }
}

#[test]
fn unavailable_reference_never_waives_candidate_execution_requirements() {
    use super::service::compare_with_contract;
    use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
    use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
    let (mut reference, valid) = matching();
    reference.execution_metadata = ExecutionMetadata::Unavailable {
        reason: "findings-only".into(),
    };
    reference.coverage = None;
    for failure in 0..5 {
        let mut candidate = valid.clone();
        match failure {
            0 => candidate.execution_metadata = ExecutionMetadata::Unknown,
            1 => candidate.coverage = None,
            2 => {
                candidate
                    .coverage
                    .as_mut()
                    .expect("fixture coverage")
                    .unknown_or_unread = 1
            }
            3 => {
                candidate
                    .coverage
                    .as_mut()
                    .expect("fixture coverage")
                    .selected = 2
            }
            _ => {
                candidate
                    .coverage
                    .as_mut()
                    .expect("fixture coverage")
                    .skipped = 1
            }
        }
        assert!(
            compare_with_contract(&reference, &candidate, ComparisonContract::Diagnostics).is_err()
        );
    }
}

#[test]
fn known_execution_difference_fails_even_when_diagnostic_multisets_match() {
    use super::service::{compare_dimensions, compare_with_contract};
    use ryo_benchmark_contracts::domain::reports::types::Comparison;
    use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
    let (reference, mut candidate) = matching();
    candidate.skipped.push(ryo::linter::Skipped {
        rule: ryo::linter::RuleId::try_from("A").expect("fixture rule"),
        path: candidate.files[0].clone(),
        reason: ryo::linter::SkipReason::SyntaxRejected,
    });
    let coverage = candidate.coverage.as_mut().expect("fixture coverage");
    coverage.evaluated = 0;
    coverage.skipped = 1;
    assert_eq!(
        compare_dimensions(&reference, &candidate).diagnostics,
        Comparison::Exact
    );
    assert_eq!(
        compare_dimensions(&reference, &candidate).execution_metadata,
        Comparison::Different
    );
    assert!(
        compare_with_contract(&reference, &candidate, ComparisonContract::Diagnostics).is_err()
    );
    assert!(compare(&reference, &candidate).is_err());
    assert!(
        compare(&candidate, &candidate).is_ok(),
        "equal explicit known skips can agree"
    );
}

#[test]
fn skipped_pairs_must_be_unique_and_belong_to_the_reported_files() {
    use super::service::require_complete_execution;
    let (_, mut candidate) = matching();
    let skip = ryo::linter::Skipped {
        rule: ryo::linter::RuleId::try_from("A").expect("fixture rule"),
        path: ryo::linter::RelativePath("unrequested.py".into()),
        reason: ryo::linter::SkipReason::SyntaxRejected,
    };
    candidate.skipped = vec![skip];
    let coverage = candidate.coverage.as_mut().expect("fixture coverage");
    coverage.evaluated = 0;
    coverage.skipped = 1;
    assert!(require_complete_execution(&candidate).is_err());
    candidate.skipped[0].path = candidate.files[0].clone();
    assert!(require_complete_execution(&candidate).is_ok());
    candidate.skipped.push(candidate.skipped[0].clone());
    let coverage = candidate.coverage.as_mut().expect("fixture coverage");
    coverage.selected = 2;
    coverage.skipped = 2;
    assert!(require_complete_execution(&candidate).is_err());
}
