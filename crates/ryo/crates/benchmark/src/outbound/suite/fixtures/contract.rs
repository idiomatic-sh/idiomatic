//! Missing implementations remain visible and an empty selection is not coverage.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{diagnostics::errors::Error, suite::types::Tool};
use crate::outbound::suite::interface::SuitePort;
type Input = bool;
type Value = (usize, usize);
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    EmptyDefaults,
    Unexpected,
}
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("missing implementation stays missing"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("empty inventory is refused"),
        word: Word::Refuses,
        input: || false,
        expect: Expect::Error(Failure::EmptyDefaults),
    },
];
pub fn check(adapter: &impl SuitePort) {
    let temporary = tempfile::tempdir().expect("suite fixture");
    std::fs::create_dir(temporary.path().join("rules")).expect("rules fixture");
    let subject = |selected| {
        let tool = Tool {
            tool: "fixture".into(),
            version: "1".into(),
            sha256: String::new(),
            defaults: if selected {
                vec!["fixture".into()]
            } else {
                Vec::new()
            },
        };
        adapter
            .audit(temporary.path(), &tool)
            .map(|audit| (audit.implemented.len(), audit.missing.len()))
            .map_err(|error| match error {
                Error::Invalid(message)
                    if message == "empty or duplicated default rule inventory" =>
                {
                    Failure::EmptyDefaults
                }
                _ => Failure::Unexpected,
            })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}

type Failure = Refusal;
