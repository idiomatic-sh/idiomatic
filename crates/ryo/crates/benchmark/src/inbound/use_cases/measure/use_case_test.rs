//! Both orderings preserve the gate and execute the requested contestant first.
use super::service::measure;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::{errors::Error, types::Diagnostic},
    measurements::types::RunOrder,
    suite::types::{Audit, RunRequest},
};
use crate::outbound::{
    execution::fixtures::fake::ExecutionFake, linter::fixtures::fake::LinterFake,
};
type Input = (bool, bool, RunOrder);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Contestant {
    Reference,
    Candidate,
}
type Value = [Contestant; 2];
#[derive(Debug, PartialEq, Eq)]
enum Failure {
    Incomplete,
    Difference,
    Unexpected,
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reference first executes reference first"),
        word: Word::Succeeds,
        input: || (true, false, RunOrder::ReferenceFirst),
        expect: Expect::Value([Contestant::Reference, Contestant::Candidate]),
    },
    Case {
        name: CaseName("candidate first executes candidate first"),
        word: Word::Succeeds,
        input: || (true, false, RunOrder::CandidateFirst),
        expect: Expect::Value([Contestant::Candidate, Contestant::Reference]),
    },
    Case {
        name: CaseName("incomplete coverage releases no observations"),
        word: Word::Refuses,
        input: || (false, false, RunOrder::ReferenceFirst),
        expect: Expect::Error(Failure::Incomplete),
    },
    Case {
        name: CaseName("mismatching findings release no observations"),
        word: Word::Refuses,
        input: || (true, true, RunOrder::CandidateFirst),
        expect: Expect::Error(Failure::Difference),
    },
];
fn subject((complete, differs, order): Input) -> Result<Value, Failure> {
    let audit = Audit {
        implemented: vec!["fixture".into()],
        missing: if complete {
            Vec::new()
        } else {
            vec!["missing".into()]
        },
    };
    let request = RunRequest {
        root: ".".into(),
        cache: "cache".into(),
        files: vec!["input.txt".into()],
    };
    let reference = LinterFake {
        arguments: vec!["reference".into()],
        ..Default::default()
    };
    let candidate = LinterFake {
        arguments: vec!["candidate".into()],
        diagnostics: if differs {
            vec![Diagnostic {
                rule: "fixture".into(),
                file: "input.txt".into(),
                start: 0,
                end: 1,
            }]
        } else {
            Vec::new()
        },
    };
    let execution = ExecutionFake {
        status: 0,
        ..Default::default()
    };
    measure(&audit, &reference, &candidate, &execution, &request, order).map_err(|error| {
        match error {
            Error::Invalid(message)
                if message == "default coverage incomplete: 1 missing rules: missing" =>
            {
                Failure::Incomplete
            }
            Error::Invalid(message) if message.starts_with("diagnostics differ:") => {
                Failure::Difference
            }
            _ => Failure::Unexpected,
        }
    })?;
    let history = execution.history.lock().expect("fixture history");
    let labels: Vec<_> = history
        .iter()
        .map(|invocation| match invocation.arguments[0].to_str() {
            Some("reference") => Ok(Contestant::Reference),
            Some("candidate") => Ok(Contestant::Candidate),
            _ => Err(Failure::Unexpected),
        })
        .collect::<Result<_, _>>()?;
    labels.try_into().map_err(|_| Failure::Unexpected)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
