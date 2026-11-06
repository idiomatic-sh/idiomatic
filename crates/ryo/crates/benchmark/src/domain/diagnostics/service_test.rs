//! Exact diagnostic multisets and submitted-file sets determine agreement.
use super::service::{compare_dimensions, compare_with_contract};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::linter::{RelativePath, RuleId, SkipReason, Skipped};
use ryo_benchmark_contracts::domain::reports::service::require_complete_execution;
use ryo_benchmark_contracts::domain::reports::types::{
    BenchmarkFinding, Comparison, ComparisonContract, ExecutionMetadata, ReportComparison,
    RuleCoverage, RunReport, SourceByteOffset,
};
use std::borrow::Cow;

/// What a case hands the part: two reports under a contract, or one report's execution accounting.
enum Check {
    Reports(ComparisonContract, RunReport, RunReport),
    Execution(RunReport),
}

/// What the part answers when the reports agree or the execution is complete.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    Compared(ReportComparison),
    Complete,
}

/// Why the part refuses: the dimensions that disagree, or the execution accounting's reason.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Differs(ReportComparison),
    Invalid(Cow<'static, str>),
}

type Input = Check;
type Value = Verdict;
type Failure = Refusal;

const INVALID_SKIPS: &str = "candidate execution contains invalid or duplicate skipped pairs";

const fn comparison(diagnostics: Comparison, execution_metadata: Comparison) -> ReportComparison {
    ReportComparison {
        diagnostics,
        execution_metadata,
    }
}

const EXACT: ReportComparison = comparison(Comparison::Exact, Comparison::Exact);
const UNPROVEN: ReportComparison = comparison(Comparison::Exact, Comparison::Incomparable);
const INVALID_EXECUTION: ReportComparison = comparison(Comparison::Exact, Comparison::Different);

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal empty findings and checked input"),
        word: Word::Succeeds,
        input: matching,
        expect: Expect::Value(Verdict::Compared(EXACT)),
    },
    Case {
        name: CaseName("an unknown reference execution is incomparable under the diagnostics contract"),
        word: Word::Succeeds,
        input: || unobservable(ExecutionMetadata::Unknown, ComparisonContract::Diagnostics),
        expect: Expect::Value(Verdict::Compared(UNPROVEN)),
    },
    Case {
        name: CaseName("an unavailable reference execution is incomparable under the diagnostics contract"),
        word: Word::Succeeds,
        input: || {
            unobservable(
                ExecutionMetadata::Unavailable {
                    reason: "findings-only format".into(),
                },
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Value(Verdict::Compared(UNPROVEN)),
    },
    Case {
        name: CaseName("equal explicit known skips can agree"),
        word: Word::Succeeds,
        input: || {
            let (_, candidate) = skipping();
            Check::Reports(
                ComparisonContract::CompleteReport,
                candidate.clone(),
                candidate,
            )
        },
        expect: Expect::Value(Verdict::Compared(EXACT)),
    },
    Case {
        name: CaseName("a skipped pair of a reported file is complete execution"),
        word: Word::Succeeds,
        input: || {
            let (_, mut candidate) = skipping();
            candidate.skipped[0].path = RelativePath("input.txt".into());
            Check::Execution(candidate)
        },
        expect: Expect::Value(Verdict::Complete),
    },
    Case {
        name: CaseName("different checked files are not agreement"),
        word: Word::Refuses,
        input: missing_file,
        expect: Expect::Error(Refusal::Differs(comparison(
            Comparison::Different,
            Comparison::Different,
        ))),
    },
    Case {
        name: CaseName("duplicate findings remain part of the multiset"),
        word: Word::Refuses,
        input: duplicate,
        expect: Expect::Error(Refusal::Differs(comparison(
            Comparison::Different,
            Comparison::Exact,
        ))),
    },
    Case {
        name: CaseName("an unknown reference execution leaves full parity unproven"),
        word: Word::Refuses,
        input: || unobservable(ExecutionMetadata::Unknown, ComparisonContract::CompleteReport),
        expect: Expect::Error(Refusal::Differs(UNPROVEN)),
    },
    Case {
        name: CaseName("an unavailable reference execution leaves full parity unproven"),
        word: Word::Refuses,
        input: || {
            unobservable(
                ExecutionMetadata::Unavailable {
                    reason: "findings-only format".into(),
                },
                ComparisonContract::CompleteReport,
            )
        },
        expect: Expect::Error(Refusal::Differs(UNPROVEN)),
    },
    Case {
        name: CaseName("an unavailable reference never waives a candidate with unknown metadata"),
        word: Word::Refuses,
        input: || unwaived(0),
        expect: Expect::Error(Refusal::Differs(INVALID_EXECUTION)),
    },
    Case {
        name: CaseName("an unavailable reference never waives a candidate without coverage"),
        word: Word::Refuses,
        input: || unwaived(1),
        expect: Expect::Error(Refusal::Differs(INVALID_EXECUTION)),
    },
    Case {
        name: CaseName("an unavailable reference never waives unread candidate pairs"),
        word: Word::Refuses,
        input: || unwaived(2),
        expect: Expect::Error(Refusal::Differs(INVALID_EXECUTION)),
    },
    Case {
        name: CaseName("an unavailable reference never waives an inconsistent selected count"),
        word: Word::Refuses,
        input: || unwaived(3),
        expect: Expect::Error(Refusal::Differs(INVALID_EXECUTION)),
    },
    Case {
        name: CaseName("an unavailable reference never waives an unaccounted skip"),
        word: Word::Refuses,
        input: || unwaived(4),
        expect: Expect::Error(Refusal::Differs(INVALID_EXECUTION)),
    },
    Case {
        name: CaseName("a known execution difference fails the diagnostics contract"),
        word: Word::Refuses,
        input: || {
            let (reference, candidate) = skipping();
            Check::Reports(ComparisonContract::Diagnostics, reference, candidate)
        },
        expect: Expect::Error(Refusal::Differs(INVALID_EXECUTION)),
    },
    Case {
        name: CaseName("a known execution difference fails the complete report contract"),
        word: Word::Refuses,
        input: || {
            let (reference, candidate) = skipping();
            Check::Reports(ComparisonContract::CompleteReport, reference, candidate)
        },
        expect: Expect::Error(Refusal::Differs(INVALID_EXECUTION)),
    },
    Case {
        name: CaseName("a skipped pair outside the reported files is invalid"),
        word: Word::Refuses,
        input: || {
            let (_, mut candidate) = skipping();
            candidate.skipped[0].path = RelativePath("unrequested.py".into());
            Check::Execution(candidate)
        },
        expect: Expect::Error(Refusal::Invalid(Cow::Borrowed(INVALID_SKIPS))),
    },
    Case {
        name: CaseName("a duplicated skipped pair is invalid"),
        word: Word::Refuses,
        input: || {
            let (_, mut candidate) = skipping();
            candidate.skipped.push(candidate.skipped[0].clone());
            let coverage = candidate.coverage.as_mut().expect("fixture coverage");
            coverage.selected = 2;
            coverage.skipped = 2;
            Check::Execution(candidate)
        },
        expect: Expect::Error(Refusal::Invalid(Cow::Borrowed(INVALID_SKIPS))),
    },
];

fn report() -> RunReport {
    RunReport {
        execution_metadata: ExecutionMetadata::Complete,
        project_context: None,
        files: vec![RelativePath("input.txt".into())],
        skipped: Vec::new(),
        coverage: Some(RuleCoverage {
            selected: 1,
            evaluated: 1,
            skipped: 0,
            unknown_or_unread: 0,
        }),
        diagnostics: Vec::new(),
    }
}

fn matching() -> Input {
    Check::Reports(ComparisonContract::CompleteReport, report(), report())
}

fn duplicate() -> Input {
    let (mut reference, mut candidate) = (report(), report());
    let finding = BenchmarkFinding {
        rule: RuleId::try_from("A").expect("fixture rule"),
        file: RelativePath("input.txt".into()),
        start: SourceByteOffset(0),
        end: SourceByteOffset(1),
    };
    reference.diagnostics = vec![finding.clone()];
    candidate.diagnostics = vec![finding.clone(), finding];
    Check::Reports(ComparisonContract::CompleteReport, reference, candidate)
}

fn missing_file() -> Input {
    let mut candidate = report();
    candidate.files.clear();
    Check::Reports(ComparisonContract::CompleteReport, report(), candidate)
}

/// A reference whose execution cannot be observed, against a valid candidate.
fn unobservable(capability: ExecutionMetadata, contract: ComparisonContract) -> Input {
    let mut reference = report();
    reference.execution_metadata = capability;
    reference.coverage = None;
    Check::Reports(contract, reference, report())
}

/// An unavailable reference against a candidate broken in the `failure`th way.
fn unwaived(failure: usize) -> Input {
    let mut reference = report();
    reference.execution_metadata = ExecutionMetadata::Unavailable {
        reason: "findings-only".into(),
    };
    reference.coverage = None;
    let mut candidate = report();
    match failure {
        0 => candidate.execution_metadata = ExecutionMetadata::Unknown,
        1 => candidate.coverage = None,
        2 => candidate.coverage.as_mut().expect("fixture coverage").unknown_or_unread = 1,
        3 => candidate.coverage.as_mut().expect("fixture coverage").selected = 2,
        _ => candidate.coverage.as_mut().expect("fixture coverage").skipped = 1,
    }
    Check::Reports(ComparisonContract::Diagnostics, reference, candidate)
}

/// A complete reference, and a candidate that explicitly skipped its one file.
fn skipping() -> (RunReport, RunReport) {
    let mut candidate = report();
    candidate.skipped.push(Skipped {
        rule: RuleId::try_from("A").expect("fixture rule"),
        path: candidate.files[0].clone(),
        reason: SkipReason::SyntaxRejected,
    });
    let coverage = candidate.coverage.as_mut().expect("fixture coverage");
    coverage.evaluated = 0;
    coverage.skipped = 1;
    (report(), candidate)
}

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Check::Reports(contract, reference, candidate) => {
            let dimensions = compare_dimensions(&reference, &candidate);
            match compare_with_contract(&reference, &candidate, contract) {
                Ok(agreed) => {
                    assert_eq!(agreed, dimensions);
                    Ok(Verdict::Compared(agreed))
                }
                Err(_difference) => Err(Refusal::Differs(dimensions)),
            }
        }
        Check::Execution(candidate) => require_complete_execution(&candidate)
            .map(|()| Verdict::Complete)
            .map_err(|error| Refusal::Invalid(error.to_string().into())),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
