//! Reference-only verification holds the reference to its exact derived expectation.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::verify_reference_cases::fixtures::authored::{
    Refusal, Shape, category, finding, history_len, prepared, project_rules, succeeding,
    verify_reference,
};
use crate::outbound::{
    execution::fixtures::fake::ExecutionFake,
    linter::fixtures::fake::LinterFake, suite::fixtures::fake::SuiteFake,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::SourceByteOffset;
use ryo_benchmark_contracts::domain::workloads::types::{CaseComparison, PreparedCase};
use std::path::Path;

/// What the reference is held to.
#[derive(Clone, Copy)]
enum Scenario {
    Multiplicity,
    Frontend,
    WrongEnd,
    Miscounted { expected_count: usize },
    NoCases,
    Startup,
}

type Input = Scenario;
type Value = usize;
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reference only accepts exact full report multiplicity without a candidate"),
        word: Word::Succeeds,
        input: || Scenario::Multiplicity,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("reference only preserves frontend findings in rule comparison"),
        word: Word::Succeeds,
        input: || Scenario::Frontend,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("reference only rejects a wrong byte end offset"),
        word: Word::Refuses,
        input: || Scenario::WrongEnd,
        expect: Expect::Error(Refusal::ReferenceMismatch),
    },
    Case {
        name: CaseName("reference only rejects an unexpected finding in an empty case"),
        word: Word::Refuses,
        input: || Scenario::Miscounted { expected_count: 0 },
        expect: Expect::Error(Refusal::ReferenceMismatch),
    },
    Case {
        name: CaseName("reference only rejects a duplicate the reference does not report"),
        word: Word::Refuses,
        input: || Scenario::Miscounted { expected_count: 2 },
        expect: Expect::Error(Refusal::ReferenceMismatch),
    },
    Case {
        name: CaseName("reference only refuses empty cases"),
        word: Word::Refuses,
        input: || Scenario::NoCases,
        expect: Expect::Error(Refusal::NoCases),
    },
    Case {
        name: CaseName("reference only refuses a reference launch failure"),
        word: Word::Refuses,
        input: || Scenario::Startup,
        expect: Expect::Error(Refusal::ReferenceStartup),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Multiplicity => multiplicity(),
        Scenario::Frontend => frontend(),
        Scenario::WrongEnd => wrong_end(),
        Scenario::Miscounted { expected_count } => miscounted(expected_count),
        Scenario::NoCases => no_cases(),
        Scenario::Startup => startup(),
    }
    .map_err(|error| category(&error.to_string()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// The reference-only verification of `suite` with a reference answering `reference`.
fn reference_only(
    suite: &SuiteFake,
    reference: &LinterFake,
    execution: &ExecutionFake,
    rules: &str,
) -> Result<Value, Error> {
    verify_reference(suite, reference, execution, Path::new(rules))
}

/// The first prepared case, compared as a full report.
fn full_case(suite: &mut SuiteFake) -> &mut PreparedCase {
    let case = suite.prepared.first_mut().expect("fixture setup must include its case");
    case.comparison = CaseComparison::Full;
    case
}

fn multiplicity() -> Result<Value, Error> {
    let (mut suite, finding) = prepared(Shape::Agreement);
    full_case(&mut suite).expected.diagnostics = vec![finding.clone(), finding.clone()];
    let reference = LinterFake {
        diagnostics: vec![finding.clone(), finding],
        ..Default::default()
    };
    let execution = succeeding();
    let result = reference_only(&suite, &reference, &execution, "rules");
    assert_eq!(history_len(&execution), Some(1));
    result
}

fn frontend() -> Result<Value, Error> {
    let (mut suite, _) = prepared(Shape::RuleProjection);
    let mut reference = LinterFake {
        diagnostics: vec![finding("E722", 0, 6), finding("F401", 20, 24)],
        ..Default::default()
    };
    project_rules(&mut suite, &mut reference);
    let execution = succeeding();
    let result = reference_only(&suite, &reference, &execution, "rules");
    assert_eq!(history_len(&execution), Some(1));
    result
}

fn wrong_end() -> Result<Value, Error> {
    let (mut suite, finding) = prepared(Shape::Agreement);
    full_case(&mut suite)
        .expected
        .diagnostics
        .first_mut()
        .expect("fixture setup must include its finding")
        .end = SourceByteOffset(7);
    let reference = LinterFake {
        diagnostics: vec![finding],
        ..Default::default()
    };
    reference_only(&suite, &reference, &succeeding(), "rules")
}

/// An authored case with `expected_count` copies of the finding, against a reference reporting it once.
fn miscounted(expected_count: usize) -> Result<Value, Error> {
    let (mut suite, finding) = prepared(Shape::Agreement);
    full_case(&mut suite).expected.diagnostics = vec![finding.clone(); expected_count];
    let reference = LinterFake {
        diagnostics: vec![finding],
        ..Default::default()
    };
    reference_only(&suite, &reference, &succeeding(), "rules")
}

fn no_cases() -> Result<Value, Error> {
    let (empty, _) = prepared(Shape::NoCases);
    let execution = succeeding();
    let result = reference_only(&empty, &LinterFake::default(), &execution, "rules");
    assert_eq!(history_len(&execution), Some(0));
    result
}

fn startup() -> Result<Value, Error> {
    let (suite, finding) = prepared(Shape::Agreement);
    let reference = LinterFake {
        diagnostics: vec![finding],
        ..Default::default()
    };
    let execution = ExecutionFake {
        status: 2,
        ..Default::default()
    };
    let result = reference_only(&suite, &reference, &execution, "rules");
    assert_eq!(history_len(&execution), Some(1));
    result
}
