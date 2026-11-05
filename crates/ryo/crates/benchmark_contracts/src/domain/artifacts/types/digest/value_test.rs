//! The exact artifact identity is carried unchanged, not recomputed by data.
use super::value::ArtifactDigest;
use ryo::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
const CASES: &[Case<&str, Cow<'static, str>, ()>] = &[Case {
    name: CaseName("digest display preserves exact identity"),
    word: Word::Succeeds,
    input: || "pin",
    expect: Expect::Value(Cow::Borrowed("pin")),
}];
fn subject(input: &str) -> Result<Cow<'static, str>, ()> {
    Ok(ArtifactDigest(input.into()).to_string().into())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
