//! The edited trial is judged on the differences accepted for the corpus before the edit: one the edit leaves passes, a new
//! one in the edited file is refused.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::measure_corpus::fixtures::edited::edited_corpus;

/// Whether the candidate reports a new finding in the file the edit changes.
type Input = bool;
/// The measurement went through.
type Value = ();
/// The line the refused project's comparison wrote.
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an accepted difference the edit leaves passes every trial"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a new difference in the edited file is refused"),
        word: Word::Fails,
        input: || true,
        expect: Expect::Error(
            "project-0: corpus comparison failed: 1 unaccepted differences: A: extra b.fixture:1:1 not accepted",
        ),
    },
];

fn subject(marks_edits: Input) -> Result<Value, Failure> {
    let mut observations = Vec::new();
    edited_corpus(marks_edits, &mut observations).map_err(|error| {
        let observations = String::from_utf8_lossy(&observations).into_owned();
        let line = observations
            .lines()
            .find(|line| line.contains("comparison failed"))
            .map_or_else(|| error.to_string(), str::to_owned);
        &*Box::leak(line.into_boxed_str())
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
