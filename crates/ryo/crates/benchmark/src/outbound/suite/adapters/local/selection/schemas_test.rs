//! What the selection document accepts and refuses, one row of `CASES` for each input.
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = &'static [u8];
type Value = usize;
type Failure = ();
/// A list of names is a selection; anything else is refused.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a list of names is a selection"),
        word: Word::Succeeds,
        input: || &br#"["one","two"]"#[..],
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("an object is not a selection"),
        word: Word::Refuses,
        input: || &br#"{"one":1}"#[..],
        expect: Expect::Error(()),
    },
];
/// Decodes one selection file and counts the names it lists.
fn subject(bytes: Input) -> Result<Value, Failure> {
    super::names(bytes)
        .map(|names| names.len())
        .map_err(|_refused| ())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
