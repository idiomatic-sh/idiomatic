//! Complete execution needs explicit accounting, even when there are no findings.
use super::{
    constants,
    service::require_complete_execution,
    types::{ExecutionMetadata, RuleCoverage, RunReport},
};
use ryo::cases::{Case, CaseName, Expect, Word, run};
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
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    let file = RelativePath("input.txt".into());
    let skip = Skipped {
        rule: RuleId::try_from("A001").expect("fixture rule"),
        path: file.clone(),
        reason: SkipReason::SyntaxRejected,
    };
    let mut report = RunReport {
        execution_metadata: ExecutionMetadata::Complete,
        project_context: None,
        diagnostics: Vec::new(),
        skipped: Vec::new(),
        coverage: Some(RuleCoverage {
            selected: 1,
            evaluated: 1,
            skipped: 0,
            unknown_or_unread: 0,
        }),
        files: vec![file.clone()],
    };
    if matches!(scenario, Scenario::MissingCoverage) {
        report.coverage = None;
        return require_complete_execution(&report).map_err(|error| error.to_string().into());
    }
    let coverage = report.coverage.as_mut().expect("complete fixture coverage");
    match scenario {
        Scenario::Complete => {}
        Scenario::KnownSkip | Scenario::ForeignSkip | Scenario::DuplicateSkip => {
            coverage.evaluated = 0;
            coverage.skipped = 1;
            report.skipped.push(skip);
            if matches!(scenario, Scenario::ForeignSkip) {
                report.skipped[0].path = RelativePath("other.txt".into());
            }
            if matches!(scenario, Scenario::DuplicateSkip) {
                report.skipped.push(report.skipped[0].clone());
                coverage.selected = 2;
                coverage.skipped = 2;
            }
        }
        Scenario::Unknown => report.execution_metadata = ExecutionMetadata::Unknown,
        Scenario::MissingCoverage => unreachable!("handled before borrowing coverage"),
        Scenario::EmptyFiles => report.files.clear(),
        Scenario::DuplicateFiles => report.files.push(file),
        Scenario::TooFewSelected => {
            coverage.selected = 0;
            coverage.evaluated = 0;
        }
        Scenario::Unread => coverage.unknown_or_unread = 1,
        Scenario::WrongTotal => coverage.selected = 2,
        Scenario::MissingSkip => {
            coverage.evaluated = 0;
            coverage.skipped = 1;
        }
        Scenario::Overflow => {
            coverage.selected = usize::MAX;
            coverage.evaluated = usize::MAX;
            coverage.skipped = 1;
            report.skipped.push(skip);
        }
    }
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
