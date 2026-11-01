//! Request validation rejects unknown and missing fields without exposing wire types.
use super::{WorkerSchemaError, request};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::ffi::OsStr;
type Input = &'static str;
type Value = bool;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    InvalidJson,
    Unexpected,
}
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unicode source path is preserved"),
        word: Word::Succeeds,
        input: || r#"{"root":"root","rules":"rules","files":["é.txt"],"cache":null}"#,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("explicit query module registry is preserved"),
        word: Word::Succeeds,
        input: || r#"{"root":"root","rules":"rules","query_modules":{"python/ruff/suppress":"fragments/suppress.ryoql"},"files":["source.py"],"cache":null}"#,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unknown request fields are rejected"),
        word: Word::Refuses,
        input: || r#"{"root":"root","rules":"rules","files":[],"extra":true}"#,
        expect: Expect::Error(Refusal::InvalidJson),
    },
    Case {
        name: CaseName("required request fields cannot be omitted"),
        word: Word::Refuses,
        input: || r#"{"root":"root","rules":"rules"}"#,
        expect: Expect::Error(Refusal::InvalidJson),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    request(OsStr::new(input))
        .map(|request| {
            let registry_matches = if input.contains("query_modules") {
                request
                    .query_modules
                    .get("python/ruff/suppress")
                    .is_some_and(|path| path == std::path::Path::new("fragments/suppress.ryoql"))
            } else {
                request.query_modules.is_empty()
            };
            registry_matches
                && (request.files == [ryo::linter::RelativePath("é.txt".into())]
                    || request.files == [ryo::linter::RelativePath("source.py".into())])
                && matches!(
                    request.state,
                    crate::domain::suite::types::WorkerState::Fresh
                )
        })
        .map_err(|error| match error {
            WorkerSchemaError::Json(_) => Refusal::InvalidJson,
            WorkerSchemaError::NotText => Refusal::Unexpected,
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
