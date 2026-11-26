//! The `token_links` service's cases: the name each kind of link is stated by.

use super::service::link_kind;
use super::types::LinkKind;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Every kind, in order.
type Input = ();
/// The names, joined by `|`.
type Value = &'static str;
/// Nothing is refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("each_kind_of_link_has_its_name"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value("next-token|matching-delimiter|enclosing-delimiter"),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let names: Vec<&str> = [LinkKind::Next, LinkKind::Matching, LinkKind::Enclosing].into_iter().map(|kind| link_kind(kind).0).collect();
    Ok(Box::leak(names.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
