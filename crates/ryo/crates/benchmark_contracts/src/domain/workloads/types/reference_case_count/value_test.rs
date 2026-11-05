//! The moved value preserves construction and exact display spelling.
use super::value::ReferenceCaseCount;
use ryo::cases::{Case, CaseName, Expect, Word, run};
const CASES: &[Case<usize, String, ()>] = &[Case {
    name: CaseName("construction preserves display spelling"),
    word: Word::Succeeds,
    input: || 7,
    expect: Expect::Value("7".into()),
}];
fn subject(input: usize) -> Result<String, ()> {
    Ok(ReferenceCaseCount(input).to_string())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
