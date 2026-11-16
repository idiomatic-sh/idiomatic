//! A bench refuses any count of processors but four before anything runs, verifies every default once, and only when they
//! pass runs the sealed worker's corpus and passes its one document through; a failed inventory runs no corpus, a stray rule
//! is refused before registering, and an absent reference tool is refused by name before anything is sealed.
use super::fixtures::{Scenario, observe};
use super::types::OperationVerdict;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operations::types::BenchOperation;
use std::borrow::Cow;

/// The scenario a row runs.
type Input = Scenario;
/// The verdict, the standard output and the artifacts requests, as text.
type Value = (OperationVerdict, Cow<'static, str>, Cow<'static, str>);
/// The refusal, the artifacts requests, as text, and whether a registration was written.
type Failure = (Cow<'static, str>, Cow<'static, str>, bool);

/// What the scripted worker writes.
const METRICS: &str = "{\"metric\":{}}\n";

/// The requests of a bench whose verification passed: the worker and the collector sealed, then the worker's corpus run.
const BENCH_CALLS: &str = "[SealWorker(\"<root>/target/benchmark/fixture/lint/bench/worker\"), SealCurrent(\"<root>/target/benchmark/fixture/lint/bench/collector\"), Run(\"<root>/target/benchmark/fixture/lint/bench/worker\", [\"measure-valid\", \"<root>/lint\", \"<root>/target/benchmark/fixture/lint/bench/state\", \"<root>/target/benchmark/fixture/lint/bench/collector\", \"--case-inventory=<root>/target/benchmark/fixture/lint/bench/registration.json\", \"--verification=<root>/target/benchmark/fixture/lint/bench/inventory\"])]";

/// The requests of a bench whose verification failed: the worker and the collector sealed, and no corpus run.
const SEALED: &str = "[SealWorker(\"<root>/target/benchmark/fixture/lint/bench/worker\"), SealCurrent(\"<root>/target/benchmark/fixture/lint/bench/collector\")]";

/// A bench on four processors whose one default the profile defers, so its verification passes.
const PASSING: Scenario = Scenario {
    operation: BenchOperation::Bench,
    tool_found: true,
    verdict: OperationVerdict::Passed,
    stray_rule: false,
    deferred: true,
    processors: 4,
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("bench passes and passes the worker's one document through"),
        word: Word::Succeeds,
        input: || PASSING,
        expect: Expect::Value((
            OperationVerdict::Passed,
            Cow::Borrowed(METRICS),
            Cow::Borrowed(BENCH_CALLS),
        )),
    },
    Case {
        name: CaseName("a failed worker fails the bench"),
        word: Word::Succeeds,
        input: || Scenario {
            verdict: OperationVerdict::Failed,
            ..PASSING
        },
        expect: Expect::Value((
            OperationVerdict::Failed,
            Cow::Borrowed(METRICS),
            Cow::Borrowed(BENCH_CALLS),
        )),
    },
    Case {
        name: CaseName("a failed inventory runs no corpus"),
        word: Word::Succeeds,
        input: || Scenario {
            deferred: false,
            ..PASSING
        },
        expect: Expect::Value((
            OperationVerdict::Failed,
            Cow::Borrowed(""),
            Cow::Borrowed(SEALED),
        )),
    },
    Case {
        name: CaseName(
            "a rule dir that is neither a default nor supporting is refused before registering",
        ),
        word: Word::Refuses,
        input: || Scenario {
            stray_rule: true,
            ..PASSING
        },
        expect: Expect::Error((
            Cow::Borrowed("rule R9 is not a shipped default"),
            Cow::Borrowed(SEALED),
            false,
        )),
    },
    Case {
        name: CaseName("an absent reference tool is refused by name before anything is sealed"),
        word: Word::Refuses,
        input: || Scenario {
            tool_found: false,
            ..PASSING
        },
        expect: Expect::Error((
            Cow::Borrowed("reference tool not found: lint"),
            Cow::Borrowed("[]"),
            false,
        )),
    },
    Case {
        name: CaseName(
            "a bench on any count of processors but four is refused naming both, before any worker is sealed",
        ),
        word: Word::Refuses,
        input: || Scenario {
            processors: 8,
            ..PASSING
        },
        expect: Expect::Error((
            Cow::Borrowed("a bench runs on 4 processors; this process may run on 8"),
            Cow::Borrowed("[]"),
            false,
        )),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let observed = observe(scenario, |executables| {
        super::fixtures::use_case_on(executables, scenario.processors)
    });
    let calls = Cow::Owned(format!("{:?}", observed.calls));
    match observed.answer {
        Ok(verdict) => Ok((
            verdict,
            Cow::Owned(String::from_utf8_lossy(&observed.stdout).into_owned()),
            calls,
        )),
        Err(refusal) => Err((Cow::Owned(refusal), calls, observed.registered)),
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
