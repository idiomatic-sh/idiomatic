//! The moved value preserves construction and exact display spelling.
use super::value::ProjectName;
use ryo::cases::{Case, CaseName, Expect, Word, run};
const CASES: &[Case<&'static str, String, ()>] = &[Case {
    name: CaseName("construction preserves display spelling"),
    word: Word::Succeeds,
    input: || "fixture",
    expect: Expect::Value("fixture".into()),
}];
fn subject(input: &'static str) -> Result<String, ()> {
    let from_borrowed = ProjectName::from(input);
    let from_owned = ProjectName::from(input.to_owned());
    assert_eq!(from_borrowed, from_owned);
    Ok(from_borrowed.to_string())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
