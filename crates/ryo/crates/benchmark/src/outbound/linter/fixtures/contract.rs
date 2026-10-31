//! Every linter prepares the requested workspace and refuses operational failure.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::measurements::types::ProcessOutput;
use crate::domain::{diagnostics::errors::Error, suite::types::RunRequest};
use crate::outbound::linter::interface::LinterPort;
use std::{path::PathBuf, time::Duration};
#[derive(Clone, Copy, Debug)]
pub enum Given {
    Prepare,
    OperationalFailure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Operational,
    Unexpected,
}
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("prepare uses requested working directory"),
        word: Word::Succeeds,
        input: || Input::Prepare,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("operational failure is not findings"),
        word: Word::Refuses,
        input: || Input::OperationalFailure,
        expect: Expect::Error(Failure::Operational),
    },
];
pub fn check(adapter: &impl LinterPort) {
    let request = RunRequest {
        root: PathBuf::new(),
        cache: PathBuf::new(),
        files: vec!["input.txt".into()],
    };
    let subject = |input| match input {
        Input::Prepare => adapter
            .prepare(&request)
            .map(|call| call.directory == request.root)
            .map_err(|_| Failure::Unexpected),
        Input::OperationalFailure => match adapter.decode(
            &request,
            ProcessOutput {
                status: 2,
                stdout: Vec::new(),
                stderr: Vec::new(),
                wall: Duration::ZERO,
            },
        ) {
            Err(Error::Invalid(_)) => Err(Failure::Operational),
            Err(_) => Err(Failure::Unexpected),
            Ok(_) => Ok(false),
        },
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}

type Failure = Refusal;

type Input = Given;

type Value = bool;
