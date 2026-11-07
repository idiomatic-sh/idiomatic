//! Receipt-only wire admission preserves all original success and missing-field controls.
use super::{
    fixtures::{Scenario, exercise},
    use_case::ValidateCaseReceiptsUseCaseImpl,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::sync::Arc;

type Input = Scenario;
type Value = usize;
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("shared receipt-only registration verifies both actual sides"),
        word: Word::Succeeds,
        input: || Scenario::Complete,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("omitted authority refuses"),
        word: Word::Refuses,
        input: || Scenario::Authority,
        expect: Expect::Error("authority"),
    },
    Case {
        name: CaseName("omitted module inventory refuses"),
        word: Word::Refuses,
        input: || Scenario::Modules,
        expect: Expect::Error("modules"),
    },
    Case {
        name: CaseName("omitted artifact inventory refuses"),
        word: Word::Refuses,
        input: || Scenario::Artifacts,
        expect: Expect::Error("artifacts"),
    },
];

fn subject(field: Input) -> Result<Value, Failure> {
    let use_case = ValidateCaseReceiptsUseCaseImpl::new(Arc::new(
        crate::outbound::inventory::factory::local(),
    ));
    exercise(&use_case, field)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
