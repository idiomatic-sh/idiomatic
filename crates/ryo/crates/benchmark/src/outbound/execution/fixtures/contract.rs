//! Execution preserves exit status and both streams; launch failure is distinct.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::errors::Error;
use crate::domain::measurements::types::Invocation;
use crate::outbound::execution::interface::ExecutionPort;
use std::path::PathBuf;
type Input = Invocation;
type Value = (i32, bool, bool);
type Failure = std::io::ErrorKind;
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("status and both streams survive"),
        word: Word::Succeeds,
        input: invocation,
        expect: Expect::Value((7, true, true)),
    },
    Case {
        name: CaseName("missing program is a launch error"),
        word: Word::Refuses,
        input: missing,
        expect: Expect::Error(std::io::ErrorKind::NotFound),
    },
];
fn invocation() -> Invocation {
    Invocation {
        executable: "sh".into(),
        arguments: vec![
            "-c".into(),
            "printf payload; printf issue >&2; exit 7".into(),
        ],
        directory: PathBuf::from("."),
    }
}
fn missing() -> Invocation {
    Invocation {
        executable: "/ryo-benchmark-missing-program".into(),
        arguments: Vec::new(),
        directory: PathBuf::from("."),
    }
}
pub fn check(adapter: &impl ExecutionPort) {
    let subject = |call| {
        adapter
            .run(call)
            .map(|output| {
                (
                    output.status,
                    output.stdout == b"payload",
                    output.stderr == b"issue",
                )
            })
            .map_err(|error| match error {
                Error::Io(error) => error.kind(),
                _ => std::io::ErrorKind::Other,
            })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
