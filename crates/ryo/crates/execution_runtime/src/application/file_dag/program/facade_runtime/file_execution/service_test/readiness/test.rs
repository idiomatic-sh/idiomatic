//! The omission falsifier runs the actual runtime and all other KernelSession calls.
use super::fixtures::rows::{Admitted, Checks, Refusal, Root, Unserved, exercise};
use super::fixtures::{EventChecks, Submission};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::StepScope;
use std::borrow::Cow;

type Input = Submission;
type Value = Admitted;
type Failure = Unserved;

/// Each authored root: a per-file region of one relation-reading head with one output kind, new
/// among the roots.
const ROOT: Root = Root {
    phase: StepScope::PerFile,
    instructions: 1,
    reads_a_relation: true,
    output_kinds: Some(1),
    new_head: true,
};

/// What every run of the corpus shows beside its obligations: only the local probe is captured,
/// five roots, each rule's replay answering its first run's cells, no events from replay, no
/// unannounced callback, and events that repeat or misplace nothing and reach every root.
const CHECKS: Checks = Checks {
    probe_events: (0, 1),
    roots: Cow::Borrowed(&[ROOT; 5]),
    runs: Cow::Borrowed(&[
        (2, true),
        (2, true),
        (2, true),
        (2, true),
        (2, true),
        (2, true),
        (4, true),
        (4, true),
        (2, true),
        (2, true),
    ]),
    replay_events: 0,
    unannounced: 0,
    events: EventChecks {
        reclassed_heads: 0,
        misplaced_ready: 0,
        repeated_ready_heads: 0,
        repeated_dispatches: 0,
        heads_as_required: true,
    },
};

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
            checks: CHECKS,
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
                checks: CHECKS,
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
