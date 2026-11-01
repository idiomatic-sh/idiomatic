//! Coverage gates require the entire pinned default inventory.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::suite::service::{require_complete, validate_cases};
use crate::domain::suite::types::audit::value::Audit;
use crate::domain::suite::types::{
    BenchmarkCaseName, Case as SuiteCase, CaseComparison, Cases, Expected, ExpectedRule,
};
type Input = Audit;
type Value = ();
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Incomplete,
    Empty,
    Unexpected,
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("complete defaults accepted"),
        word: Word::Succeeds,
        input: || Audit {
            implemented: vec![ryo::linter::RuleId::try_from("fixture").unwrap()],
            missing: Vec::new(),
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("one missing default prevents measurement"),
        word: Word::Refuses,
        input: || Audit {
            implemented: Vec::new(),
            missing: vec![ryo::linter::RuleId::try_from("fixture").unwrap()],
        },
        expect: Expect::Error(Failure::Incomplete),
    },
    Case {
        name: CaseName("empty internal audit is refused"),
        word: Word::Refuses,
        input: || Audit {
            implemented: Vec::new(),
            missing: Vec::new(),
        },
        expect: Expect::Error(Failure::Empty),
    },
];
#[expect(
    clippy::needless_pass_by_value,
    reason = "the shared case runner hands each independently constructed input to the subject by value"
)]
fn subject(input: Input) -> Result<Value, Failure> {
    require_complete(&input).map_err(|error| match error {
        Error::Invalid(message)
            if message == "default coverage incomplete: 1 missing rules: fixture" =>
        {
            Failure::Incomplete
        }
        Error::Invalid(message) if message == "empty audit is not default coverage" => {
            Failure::Empty
        }
        Error::Invalid(_) | Error::Io(_) | Error::Json(_) | Error::Toml(_) => Failure::Unexpected,
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

#[test]
fn full_report_case_polarity_is_relative_to_the_enclosing_rule() {
    let rule = ryo::linter::RuleId::try_from("F601").expect("rule identity");
    let other_rule = ryo::linter::RuleId::try_from("B017").expect("other rule identity");
    let file = ryo::linter::RelativePath("case.py".into());
    let expected = |identity| Expected {
        identity: ExpectedRule::Explicit(identity),
        file: file.clone(),
        start: crate::domain::diagnostics::types::SourceByteOffset(0),
        end: crate::domain::diagnostics::types::SourceByteOffset(1),
    };
    let source =
        || std::collections::BTreeMap::from([(file.clone(), ryo::linter::SourceText("x".into()))]);
    let cases = Cases {
        cases: vec![
            SuiteCase {
                comparison: CaseComparison::Full,
                name: BenchmarkCaseName("positive".into()),
                files: source(),
                expected: vec![expected(rule.clone()), expected(rule.clone())],
            },
            SuiteCase {
                comparison: CaseComparison::Full,
                name: BenchmarkCaseName("other-rule-only".into()),
                files: source(),
                expected: vec![expected(other_rule)],
            },
        ],
    };
    assert!(validate_cases(&rule, &cases).is_ok());
    assert_eq!(
        cases.cases[0].expected.len(),
        2,
        "duplicate findings remain significant"
    );

    let missing_negative = Cases {
        cases: vec![cases.cases[0].clone()],
    };
    assert!(matches!(
        validate_cases(&rule, &missing_negative),
        Err(Error::Invalid(message)) if message == super::constants::MISSING_POLARITY
    ));
}

type Failure = Refusal;
