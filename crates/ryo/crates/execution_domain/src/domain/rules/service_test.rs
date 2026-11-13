//! The `rules` service's cases, on values alone.

use std::borrow::Cow;

use super::service::id_at;
use crate::domain::coverage::types::RuleIndex;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::runs::types::RuleId;

/// The ids of the rules of a run, in their order, and the place asked for.
#[derive(Debug)]
struct Place {
    ids: &'static [&'static str],
    at: usize,
}

type Input = Place;
/// The id of the rule at the place.
type Value = Cow<'static, str>;
type Failure = std::convert::Infallible;

fn subject(place: Input) -> Result<Value, Failure> {
    let ids: Vec<RuleId> = place.ids.iter().map(|id| RuleId((*id).into())).collect();
    Ok(Cow::Owned(id_at(&ids, RuleIndex(place.at)).0.into()))
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_id_at_a_place_is_the_id_of_the_rule_there"),
    word: Word::Succeeds,
    input: || Place {
        ids: &["a", "b", "c"],
        at: 2,
    },
    expect: Expect::Value(Cow::Borrowed("c")),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
