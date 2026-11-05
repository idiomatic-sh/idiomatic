//! The exact artifact identity is carried unchanged, not recomputed by data.
use super::value::ArtifactDigest;
use ryo::cases::{Case, CaseName, Expect, Word, run};
const CASES: &[Case<&str, String, ()>] = &[Case {
    name: CaseName("digest display preserves exact identity"),
    word: Word::Succeeds,
    input: || "pin",
    expect: Expect::Value("pin".into()),
}];
fn subject(input: &str) -> Result<String, ()> {
    Ok(ArtifactDigest(input.into()).to_string())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
