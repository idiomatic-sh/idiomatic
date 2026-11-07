//! The moved value preserves construction and exact display spelling.
use super::value::ToolDigest;
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
const CASES: &[Case<&'static str, Cow<'static, str>, ()>] = &[Case {
    name: CaseName("construction preserves display spelling"),
    word: Word::Succeeds,
    input: || "fixture",
    expect: Expect::Value(Cow::Borrowed("fixture")),
}];
fn subject(input: &'static str) -> Result<Cow<'static, str>, ()> {
    let from_borrowed = ToolDigest::from(input);
    let from_owned = ToolDigest::from(input.to_owned());
    assert_eq!(from_borrowed, from_owned);
    Ok(from_borrowed.to_string().into())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
