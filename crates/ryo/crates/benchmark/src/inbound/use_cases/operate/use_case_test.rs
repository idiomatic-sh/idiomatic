//! Parity runs the sealed worker on its output directory and passes its output through; a failed worker fails the
//! operation, and an absent reference tool is refused by name before anything is sealed.
use super::fixtures::{Scenario, run as observe};
use super::types::OperationVerdict;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operations::types::Operation;
use std::borrow::Cow;

/// The verdict, the standard output and the artifacts requests, as text.
type Value = (OperationVerdict, Cow<'static, str>, Cow<'static, str>);
/// The refusal and the artifacts requests, as text.
type Failure = (Cow<'static, str>, Cow<'static, str>);

/// What the scripted worker writes.
const METRICS: &str = "{\"metric\":{}}\n";

/// The requests of a parity run: the worker sealed in its output directory, then run on the reference and that directory.
const PARITY_CALLS: &str = "[SealWorker(\"<root>/target/benchmark/fixture/lint/parity/worker\"), Run(\"<root>/target/benchmark/fixture/lint/parity/worker\", [\"parity\", \"<root>/lint\", \"<root>/target/benchmark/fixture/lint/parity\"])]";

const CASES: &[Case<Scenario, Value, Failure>] = &[
    Case {
        name: CaseName("parity seals the worker and runs it on the reference and its output directory"),
        word: Word::Succeeds,
        input: || Scenario {
            operation: Operation::Parity,
            tool_found: true,
            verdict: OperationVerdict::Passed,
        },
        expect: Expect::Value((
            OperationVerdict::Passed,
            Cow::Borrowed(METRICS),
            Cow::Borrowed(PARITY_CALLS),
        )),
    },
    Case {
        name: CaseName("a failed worker fails the operation"),
        word: Word::Succeeds,
        input: || Scenario {
            operation: Operation::Parity,
            tool_found: true,
            verdict: OperationVerdict::Failed,
        },
        expect: Expect::Value((
            OperationVerdict::Failed,
            Cow::Borrowed(METRICS),
            Cow::Borrowed(PARITY_CALLS),
        )),
    },
    Case {
        name: CaseName("an absent reference tool is refused by name before anything is sealed"),
        word: Word::Refuses,
        input: || Scenario {
            operation: Operation::Parity,
            tool_found: false,
            verdict: OperationVerdict::Passed,
        },
        expect: Expect::Error((
            Cow::Borrowed("reference tool not found: lint"),
            Cow::Borrowed("[]"),
        )),
    },
];

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    let observed = observe(scenario);
    let calls = Cow::Owned(format!("{:?}", observed.calls));
    match observed.answer {
        Ok(verdict) => Ok((
            verdict,
            Cow::Owned(String::from_utf8_lossy(&observed.stdout).into_owned()),
            calls,
        )),
        Err(refusal) => Err((Cow::Owned(refusal), calls)),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
