//! The moved value preserves construction and exact display spelling.
use super::value::ReferenceCaseCount;
use ryo::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
const CASES: &[Case<usize, Cow<'static, str>, ()>] = &[Case {
    name: CaseName("construction preserves display spelling"),
    word: Word::Succeeds,
    input: || 7,
    expect: Expect::Value(Cow::Borrowed("7")),
}];
fn subject(input: usize) -> Result<Cow<'static, str>, ()> {
    Ok(ReferenceCaseCount(input).to_string().into())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
