//! The omission falsifier runs the actual runtime and all other KernelSession calls.
use super::fixtures::Submission;
use super::fixtures::rows::{Admitted, Refusal, Unserved, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = Submission;
type Value = Admitted;
type Failure = Unserved;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("runtime obligations precede actual ready callbacks and equivalent replay"),
        word: Word::Succeeds,
        input: || Submission::Actual,
        expect: Expect::Value(Admitted {
            dispatches: 2,
            obligations: 4,
            heads: 5,
            skipped: 0,
        }),
    },
    Case {
        name: CaseName(
            "omitting one actual ready submission preserves its obligation while other dispatches continue",
        ),
        word: Word::Rejects,
        input: || Submission::Omitted,
        expect: Expect::Error(Unserved {
            refusal: Refusal::MissingSubmission,
            admitted: Admitted {
                dispatches: 2,
                obligations: 4,
                heads: 5,
                skipped: 1,
            },
            unserved: 1,
        }),
    },
];

/// Admits the corpus through the ready adapter and compares its obligations with those served.
fn subject(submission: Input) -> Result<Value, Failure> {
    exercise(submission)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn rejects() {
    run(CASES, Word::Rejects, subject);
}
