//! The `names` service's cases: the reasons are declared in their order.

use super::service::unknown;
use super::types::Unresolved;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::language::constants::UNKNOWN_REASONS;
use ryo::languages;
use strum::IntoEnumIterator as _;

/// Nothing is given: the reasons are the subject.
type Input = ();
/// Whether the reasons are declared in their order, each at its number.
type Value = bool;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_reasons_are_declared_in_their_order_each_at_its_number"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let reasons: Vec<&'static str> = Unresolved::iter().map(<&'static str>::from).collect();
    let declared: Vec<&'static str> = UNKNOWN_REASONS.iter().map(|each| each.text.0).collect();
    let numbered = Unresolved::iter()
        .zip(UNKNOWN_REASONS)
        .all(|(reason, each)| unknown(reason) == languages::unknown_symbol(each.reason));
    if !numbered
        || declared.get(..reasons.len()) != Some(reasons.as_slice())
        || declared.len() != reasons.len() + 1
        || declared.last() != Some(&"semantic-rendering-unsupported")
        || UNKNOWN_REASONS[reasons.len()].reason != languages::UnknownReason(24)
        || unknown(Unresolved::WildcardImport) == unknown(Unresolved::StarBase)
    {
        return Err(());
    }
    Ok(true)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
