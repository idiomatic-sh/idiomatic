//! The public operation seals evidence and refuses changed inputs.
use super::fixtures::{Change, exercise};
use super::use_case::CertifyUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = Change;
type Value = crate::domain::suite::types::CaseCount;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unchanged certified workload measures"),
        word: Word::Succeeds,
        input: || Change::None,
        expect: Expect::Value(crate::domain::suite::types::CaseCount(2)),
    },
    Case {
        name: CaseName("patch mutation refuses measurement"),
        word: Word::Refuses,
        input: || Change::Patch,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("project pin mutation refuses measurement"),
        word: Word::Refuses,
        input: || Change::Manifest,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("changed collector refuses before execution"),
        word: Word::Refuses,
        input: || Change::Collector,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("query mutation refuses measurement"),
        word: Word::Refuses,
        input: || Change::Query,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("executable mutation refuses measurement"),
        word: Word::Refuses,
        input: || Change::Executable,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("invocation change refuses measurement"),
        word: Word::Refuses,
        input: || Change::Command,
        expect: Expect::Error(()),
    },
];
#[test]
#[cfg(unix)]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
#[cfg(unix)]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input, CertifyUseCaseImpl::new)
}
