//! Every absence cause has its own stable displayed name.

use super::service::reason_name;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::answers::types::AbsenceCause;

type Input = fn() -> usize;
type Value = usize;
type Failure = ();

fn every_reason_has_a_name_of_its_own() -> usize {
    let mut names: Vec<&str> = [
        AbsenceCause::NoSuchConstruct,
        AbsenceCause::NotCode,
        AbsenceCause::NotAKeyedDocument,
        AbsenceCause::NoManifests,
    ]
    .into_iter()
    .map(|reason| reason_name(reason).0)
    .collect();
    names.sort_unstable();
    names.dedup();
    names.len()
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_reason_has_a_name_of_its_own"),
    word: Word::Succeeds,
    input: || every_reason_has_a_name_of_its_own,
    expect: Expect::Value(4),
}];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
