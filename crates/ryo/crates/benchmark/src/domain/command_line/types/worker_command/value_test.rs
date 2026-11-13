//! A worker command names its tracing; any other command is no worker protocol.
use super::value::WorkerCommand;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = &'static str;
/// The tracing, rendered as text.
type Value = &'static str;

const CASES: &[Case<Input, Value, ()>] = &[
    Case {
        name: CaseName("the timed worker command traces nothing"),
        word: Word::Succeeds,
        input: || "run-ryo",
        expect: Expect::Value("Some(Disabled)"),
    },
    Case {
        name: CaseName("the profiling worker command writes a Chrome trace"),
        word: Word::Succeeds,
        input: || "profile-ryo",
        expect: Expect::Value("Some(Chrome)"),
    },
    Case {
        name: CaseName("the diagnostic worker command writes JSONL events"),
        word: Word::Succeeds,
        input: || "diagnostic-ryo",
        expect: Expect::Value("Some(JsonLines)"),
    },
    Case {
        name: CaseName("another command is no worker protocol"),
        word: Word::Succeeds,
        input: || "diagnostic-unknown",
        expect: Expect::Value("None"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |command| {
        Ok(&*Box::leak(
            format!("{:?}", WorkerCommand(command).tracing()).into_boxed_str(),
        ))
    });
}
