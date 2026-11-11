//! The support of the `kind_set` cases.
use crate::domain::table_model::types::SetMembership;
use crate::domain::table_scratch::service::kind_has;
use crate::domain::table_scratch::types::{KindMask, KindSet};
use ryo_relations::domain::relations::types::KindBits;
use ryo_values::cases::{Case, CaseName, Expect, Word};

/// A scenario: what a row asks of the part.
pub(super) type Input = fn() -> Result<Value, ()>;

/// What a scenario answers: a yes as one.
type Value = usize;

/// This part has no failure of its own.
pub(super) type Failure = ();

/// A kind is in a set when its bit is, and a kind beyond the set is out.
pub(super) fn a_kind_is_in_the_set_by_its_bit() -> Value {
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
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_kind_is_in_the_set_by_its_bit"),
    word: Word::Succeeds,
    input: || || Ok(a_kind_is_in_the_set_by_its_bit()),
    expect: Expect::Value(1),
}];

pub(super) fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}
