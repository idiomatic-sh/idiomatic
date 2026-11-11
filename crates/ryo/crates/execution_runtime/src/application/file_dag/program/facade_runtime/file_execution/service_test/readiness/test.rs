//! The omission falsifier runs the actual runtime and all other KernelSession calls.
use super::fixtures::{Coverage, observe};
use crate::cases::{Case, CaseName, Expect, Word, run};
#[derive(Clone, Copy)]
enum Scenario {
    Actual,
    Omitted,
}
#[derive(Debug, PartialEq)]
enum Refusal {
    MissingSubmission,
}
type Input = Scenario;
type Value = ();
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("runtime obligations precede actual ready callbacks and equivalent replay"),
        word: Word::Succeeds,
        input: || Scenario::Actual,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "omitting one actual ready submission preserves its obligation while other dispatches continue",
        ),
        word: Word::Rejects,
        input: || Scenario::Omitted,
        expect: Expect::Error(Refusal::MissingSubmission),
    },
];
fn verify(observed: Coverage) -> Result<(), Refusal> {
    assert!(
        observed.dispatches >= 2,
        "both actual source populations still dispatch"
    );
    assert!(observed.expected.len() > 1);
    assert!(
        observed.heads > observed.expected.len(),
        "different physical heads can share one admitted class"
    );
    if observed.expected != observed.served {
        return Err(Refusal::MissingSubmission);
    }
    Ok(())
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    let omit = matches!(scenario, Scenario::Omitted);
    let observed = observe(omit);
    assert_eq!(
        observed.skipped,
        usize::from(omit),
        "the adapter omits exactly one actual submission"
    );
    verify(observed)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn rejects() {
    run(CASES, Word::Rejects, subject);
}
