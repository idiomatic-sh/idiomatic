//! Which side of a row an identity belongs to orders reference before candidate.
use super::value::{IntentMatch, RecordedReason};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::documents::types::text::value::{ContestantSide, ReasonText, YesNo};
use std::cmp::Ordering;

#[derive(Clone, Copy)]
enum Scenario {
    SidesOrder,
    ReasonKeepsWords,
    IntentKeepsAnswer,
}
type Input = Scenario;
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the reference side orders before the candidate"),
        word: Word::Succeeds,
        input: || Scenario::SidesOrder,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a stated reason keeps its words"),
        word: Word::Succeeds,
        input: || Scenario::ReasonKeepsWords,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a matched intent keeps its answer apart from an unreported one"),
        word: Word::Succeeds,
        input: || Scenario::IntentKeepsAnswer,
        expect: Expect::Value(true),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(match scenario {
        Scenario::SidesOrder => {
            ContestantSide::Reference.cmp(&ContestantSide::Candidate) == Ordering::Less
        }
        Scenario::ReasonKeepsWords => {
            RecordedReason::Stated(ReasonText("why".into())) != RecordedReason::Absent
        }
        Scenario::IntentKeepsAnswer => IntentMatch::Matched(YesNo::No) != IntentMatch::Unreported,
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
