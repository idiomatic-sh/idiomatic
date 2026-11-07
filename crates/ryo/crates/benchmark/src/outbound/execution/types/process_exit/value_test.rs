//! A child's exit keeps its code, and a signal ending it stays a signal.
use super::value::{ProcessExit, decode};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::measurements::types::{
    ProcessExitCode, SignalNumber, TerminationSignal,
};

type Input = &'static str;
type Value = ProcessExit;
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a numeric exit keeps its code and reports no signal"),
        word: Word::Succeeds,
        input: || "exit 3",
        expect: Expect::Value(ProcessExit {
            code: ProcessExitCode(3),
            termination: TerminationSignal::Exited,
        }),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("a signaled child keeps its signal and a negative compatibility code"),
        word: Word::Succeeds,
        input: || "kill -TERM $$",
        expect: Expect::Value(ProcessExit {
            code: ProcessExitCode(-15),
            termination: TerminationSignal::Signalled(ryo_benchmark_contracts::domain::measurements::types::SignalNumber(15)),
        }),
    },
];
fn subject(script: Input) -> Result<Value, Failure> {
    let status = std::process::Command::new("sh")
        .args(["-c", script])
        .status()
        .expect("shell fixture");
    Ok(decode(status).expect("a completed child has a status"))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
