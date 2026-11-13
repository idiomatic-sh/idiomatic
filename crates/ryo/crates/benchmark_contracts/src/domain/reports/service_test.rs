//! Complete execution needs explicit accounting, even when there are no findings.
use super::{
    constants,
    errors::ReportsError,
    service::require_complete_execution,
    types::{
        CoverageAccounting, ExecutionMetadata, PairCount, ProjectContext, RuleCoverage, RunReport,
    },
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::linter::{RelativePath, RuleId, SkipReason, Skipped};

#[derive(Clone, Copy)]
enum Scenario {
    Complete,
    KnownSkip,
    Unknown,
    MissingCoverage,
    EmptyFiles,
    DuplicateFiles,
    TooFewSelected,
    Unread,
    WrongTotal,
    MissingSkip,
    ForeignSkip,
    DuplicateSkip,
    Overflow,
    InvalidReason,
}
type Input = Scenario;
type Value = ();
type Failure = std::borrow::Cow<'static, str>;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("complete evaluated file"),
        word: Word::Succeeds,
        input: || Scenario::Complete,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("explicit syntax skip accounts for file"),
        word: Word::Succeeds,
        input: || Scenario::KnownSkip,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unknown capability cannot establish completeness"),
        word: Word::Refuses,
        input: || Scenario::Unknown,
        expect: Expect::Error(std::borrow::Cow::Borrowed(constants::INCOMPLETE_METADATA)),
    },
    Case {
        name: CaseName("coverage cannot be inferred from findings"),
        word: Word::Refuses,
        input: || Scenario::MissingCoverage,
        expect: Expect::Error(std::borrow::Cow::Borrowed(constants::MISSING_COVERAGE)),
    },
    Case {
        name: CaseName("empty file inventory refuses"),
        word: Word::Refuses,
        input: || Scenario::EmptyFiles,
        expect: Expect::Error(std::borrow::Cow::Borrowed(constants::INCONSISTENT_COVERAGE)),
    },
    Case {
        name: CaseName("duplicate files refuse"),
        word: Word::Refuses,
        input: || Scenario::DuplicateFiles,
        expect: Expect::Error(std::borrow::Cow::Borrowed(constants::INCONSISTENT_COVERAGE)),
    },
    Case {
        name: CaseName("every file needs a selected pair"),
        word: Word::Refuses,
        input: || Scenario::TooFewSelected,
        expect: Expect::Error(std::borrow::Cow::Borrowed(constants::INCONSISTENT_COVERAGE)),
    },
    Case {
        name: CaseName("unread work cannot establish completeness"),
        word: Word::Refuses,
        input: || Scenario::Unread,
        expect: Expect::Error(std::borrow::Cow::Borrowed(constants::INCONSISTENT_COVERAGE)),
    },
    Case {
        name: CaseName("evaluated and skipped must sum to selection"),
        word: Word::Refuses,
        input: || Scenario::WrongTotal,
        expect: Expect::Error(std::borrow::Cow::Borrowed(constants::INCONSISTENT_COVERAGE)),
    },
    Case {
        name: CaseName("skip count needs actual skip identities"),
        word: Word::Refuses,
        input: || Scenario::MissingSkip,
        expect: Expect::Error(std::borrow::Cow::Borrowed(constants::INCONSISTENT_COVERAGE)),
    },
    Case {
        name: CaseName("skip outside submitted files refuses"),
        word: Word::Refuses,
        input: || Scenario::ForeignSkip,
        expect: Expect::Error(std::borrow::Cow::Borrowed(constants::INVALID_SKIPS)),
    },
    Case {
        name: CaseName("duplicate skip identities refuse"),
        word: Word::Refuses,
        input: || Scenario::DuplicateSkip,
        expect: Expect::Error(std::borrow::Cow::Borrowed(constants::INVALID_SKIPS)),
    },
    Case {
        name: CaseName("overflow cannot establish equal counts"),
        word: Word::Refuses,
        input: || Scenario::Overflow,
        expect: Expect::Error(std::borrow::Cow::Borrowed(constants::INCONSISTENT_COVERAGE)),
    },
    Case {
        name: CaseName("an invalid observation keeps its reason and displays it unchanged"),
        word: Word::Succeeds,
        input: || Scenario::InvalidReason,
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    if matches!(scenario, Scenario::InvalidReason) {
        let error = ReportsError::Invalid("coverage is missing".into());
        let message = error.to_string();
        return match error {
            ReportsError::Invalid(reason)
                if reason == message && reason == "coverage is missing" =>
            {
                Ok(())
            }
            other @ (ReportsError::Invalid(_)
            | ReportsError::Io(_)
            | ReportsError::Json(_)
            | ReportsError::Toml(_)) => Err(other.to_string().into()),
        };
    }
    let file = RelativePath("input.txt".into());
    let skip = Skipped {
        rule: RuleId::try_from("A001").expect("fixture rule"),
        path: file.clone(),
        reason: SkipReason::SyntaxRejected,
    };
    let mut report = RunReport {
        execution_metadata: ExecutionMetadata::Complete,
        project_context: ProjectContext::NotRequested,
        diagnostics: Vec::new(),
        skipped: Vec::new(),
        coverage: CoverageAccounting::Accounted(RuleCoverage {
            selected: PairCount(1),
            evaluated: PairCount(1),
            skipped: PairCount(0),
            unknown_or_unread: PairCount(0),
        }),
        files: vec![file.clone()],
    };
    if matches!(scenario, Scenario::MissingCoverage) {
        report.coverage = CoverageAccounting::Unaccounted;
        return require_complete_execution(&report).map_err(|error| error.to_string().into());
    }
    let CoverageAccounting::Accounted(mut coverage) = report.coverage else {
        return Err("complete fixture coverage".into());
    };
    match scenario {
        Scenario::Complete => {}
        Scenario::KnownSkip | Scenario::ForeignSkip | Scenario::DuplicateSkip => {
            coverage.evaluated = PairCount(0);
            coverage.skipped = PairCount(1);
            report.skipped.push(skip);
            if matches!(scenario, Scenario::ForeignSkip) {
                report.skipped[0].path = RelativePath("other.txt".into());
            }
            if matches!(scenario, Scenario::DuplicateSkip) {
                report.skipped.push(report.skipped[0].clone());
                coverage.selected = PairCount(2);
                coverage.skipped = PairCount(2);
            }
        }
        Scenario::Unknown => report.execution_metadata = ExecutionMetadata::Unknown,
        Scenario::MissingCoverage => unreachable!("handled before borrowing coverage"),
        Scenario::EmptyFiles => report.files.clear(),
        Scenario::DuplicateFiles => report.files.push(file),
        Scenario::TooFewSelected => {
            coverage.selected = PairCount(0);
            coverage.evaluated = PairCount(0);
        }
        Scenario::Unread => coverage.unknown_or_unread = PairCount(1),
        Scenario::WrongTotal => coverage.selected = PairCount(2),
        Scenario::MissingSkip => {
            coverage.evaluated = PairCount(0);
            coverage.skipped = PairCount(1);
        }
        Scenario::InvalidReason => unreachable!("handled before the report is built"),
        Scenario::Overflow => {
            coverage.selected = PairCount(usize::MAX);
            coverage.evaluated = PairCount(usize::MAX);
            coverage.skipped = PairCount(1);
            report.skipped.push(skip);
        }
    }
    report.coverage = CoverageAccounting::Accounted(coverage);
    require_complete_execution(&report).map_err(|error| error.to_string().into())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
