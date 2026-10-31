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
            request.files == [ryo::linter::RelativePath("é.txt".into())]
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
