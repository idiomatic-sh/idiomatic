//! A child's exit keeps its code, and a signal ending it stays a signal.
use super::value::{ProcessExit, decode};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::execution::errors::ExecutionError;
use ryo_benchmark_contracts::domain::measurements::types::{
    ProcessExitCode, SignalNumber, TerminationSignal,
};

/// A child that ended, or a status the platform reports without a code or a signal.
#[derive(Clone, Copy, Debug)]
enum Child {
    /// A shell run to completion with this script.
    Shell(&'static str),
    /// A child that is stopped, not ended: neither an exit code nor a signal.
    #[cfg(unix)]
    Stopped,
}
type Input = Child;
type Value = ProcessExit;
type Failure = ExecutionError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a numeric exit keeps its code and reports no signal"),
        word: Word::Succeeds,
        input: || Child::Shell("exit 3"),
        expect: Expect::Value(ProcessExit {
            code: ProcessExitCode(3),
            termination: TerminationSignal::Exited,
        }),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("a signaled child keeps its signal and a negative compatibility code"),
        word: Word::Succeeds,
        input: || Child::Shell("kill -TERM $$"),
        expect: Expect::Value(ProcessExit {
            code: ProcessExitCode(-15),
            termination: TerminationSignal::Signalled(SignalNumber(15)),
        }),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("a status with neither a code nor a signal is refused"),
        word: Word::Refuses,
        input: || Child::Stopped,
        expect: Expect::Error(ExecutionError::UnobservedExit),
    },
];
#[expect(
    clippy::expect_used,
    reason = "a shell fixture that cannot start must fail the test"
)]
fn subject(child: Input) -> Result<Value, Failure> {
    match child {
        Child::Shell(script) => decode(
            std::process::Command::new("sh")
                .args(["-c", script])
                .status()
                .expect("shell fixture"),
        ),
        #[cfg(unix)]
        Child::Stopped => {
            use std::os::unix::process::ExitStatusExt as _;
            decode(std::process::ExitStatus::from_raw(0x137f))
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[cfg(unix)]
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
