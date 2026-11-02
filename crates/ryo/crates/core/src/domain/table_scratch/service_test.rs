//! The `table_scratch` service's cases, on values alone.

use ryo_language::domain::relations::types::KindBits;

use super::service::kind_has;
use super::types::{KindMask, KindSet};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_model::types::SetMembership;

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, ()>;
/// What a scenario answers: a yes as one.
type Value = usize;
/// This part has no failure of its own.
type Failure = ();

/// A kind is in a set when its bit is, and a kind beyond the set is out.
fn a_kind_is_in_the_set_by_its_bit() -> Value {
    let set = KindSet {
        bits: KindMask(vec![0b100]),
        kinds: vec![KindBits(2)],
    };
    usize::from(
        kind_has(&set, KindBits(2)) == SetMembership::In
            && kind_has(&set, KindBits(3)) == SetMembership::Out
            && kind_has(&set, KindBits(130)) == SetMembership::Out,
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_kind_is_in_the_set_by_its_bit"),
    word: Word::Succeeds,
    input: || || Ok(a_kind_is_in_the_set_by_its_bit()),
    expect: Expect::Value(1),
}];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
