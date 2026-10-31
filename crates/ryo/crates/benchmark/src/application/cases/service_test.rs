//! Neither contestant can certify a case by silently dropping its expected finding.
use super::service::verify;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::types::{Diagnostic, RunReport},
    suite::types::{PreparedCase, RunRequest},
};
use crate::outbound::{
    execution::fixtures::fake::ExecutionFake, linter::fixtures::fake::LinterFake,
    suite::fixtures::fake::SuiteFake,
};
use std::path::Path;

#[derive(Clone, Copy)]
enum Scenario {
    Agreement,
    MissingCandidate,
    MissingReference,
    NoCases,
}
const CASES: &[Case<Scenario, usize, &'static str>] = &[
    Case {
        name: CaseName("both contestants establish the independent expectation"),
        word: Word::Succeeds,
        input: || Scenario::Agreement,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("candidate omissions cannot certify"),
        word: Word::Refuses,
        input: || Scenario::MissingCandidate,
        expect: Expect::Error("candidate mismatch"),
    },
    Case {
        name: CaseName("reference omissions cannot certify"),
        word: Word::Refuses,
        input: || Scenario::MissingReference,
        expect: Expect::Error("reference mismatch"),
    },
    Case {
        name: CaseName("empty case sources refuse certification"),
        word: Word::Refuses,
        input: || Scenario::NoCases,
        expect: Expect::Error("no cases"),
    },
];
fn subject(scenario: Scenario) -> Result<usize, &'static str> {
    let expected = Diagnostic {
        rule: "E722".into(),
        file: "case.py".into(),
        start: 0,
        end: 6,
    };
    let mut suite = SuiteFake::default();
    if !matches!(scenario, Scenario::NoCases) {
        suite.prepared.push(PreparedCase {
            rule: "E722".into(),
            name: "positive".into(),
            request: RunRequest {
                root: ".".into(),
                files: vec!["case.py".into()],
                cache: "cache".into(),
            },
            expected: RunReport {
                files: vec!["case.py".into()],
                diagnostics: vec![expected.clone()],
            },
        });
    }
    let reference = LinterFake {
        diagnostics: if matches!(scenario, Scenario::MissingReference) {
            Vec::new()
        } else {
            vec![expected.clone()]
        },
        ..Default::default()
    };
    let candidate = LinterFake {
        diagnostics: if matches!(scenario, Scenario::MissingCandidate) {
            Vec::new()
        } else {
            vec![expected]
        },
        ..Default::default()
    };
    let execution = ExecutionFake {
        status: 0,
        ..Default::default()
    };
    let result = verify(
        &suite,
        &reference,
        &candidate,
        &execution,
        Path::new("rules"),
        Path::new("scratch"),
    );
    if !matches!(scenario, Scenario::NoCases) {
        assert_eq!(execution.history.lock().expect("history").len(), 2);
    }
    result.map_err(|error| {
        let message = error.to_string();
        if message.contains("expected-vs-candidate") {
            "candidate mismatch"
        } else if message.contains("expected-vs-reference") {
            "reference mismatch"
        } else if message == "no executable rule cases" {
            "no cases"
        } else {
            "unexpected error"
        }
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
