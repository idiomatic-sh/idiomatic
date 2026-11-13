//! The line keeps the exact spelling it was given.
use super::value::VersionLine;
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
const CASES: &[Case<&'static str, Cow<'static, str>, ()>] = &[Case {
    name: CaseName("display is the exact line"),
    word: Word::Succeeds,
    input: || "Version: 1.76.0",
    expect: Expect::Value(Cow::Borrowed("Version: 1.76.0")),
}];
fn subject(input: &'static str) -> Result<Cow<'static, str>, ()> {
    Ok(VersionLine(input.into()).to_string().into())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
