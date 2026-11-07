//! The `tokens` service's cases: each kind is named in kebab case, every name once.

use super::service::standard;
use super::types::Token;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::kind_name;
use crate::domain::syntax::types::Kind;
use strum::IntoEnumIterator as _;

/// The page of the pinned standards, which names each standard and its version.
const PINNED: &str = include_str!("../../../tests/fixtures/standard/README.md");

/// One kind.
type Input = Token;
/// Its name, and whether the pinned standard's page names the standard; a name given twice fails the row.
type Value = (&'static str, bool);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_kind_is_named_in_kebab_case_and_every_name_once"),
    word: Word::Succeeds,
    input: || Token::FstringStart,
    expect: Expect::Value(("fstring-start", true)),
}];

fn subject(kind: Input) -> Result<Value, Failure> {
    let mut names: Vec<&str> = Token::iter()
        .map(|each| kind_name(Kind::Token(each)).0)
        .collect();
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    if names.len() != count {
        return Err(());
    }
    Ok((
        kind_name(Kind::Token(kind)).0,
        PINNED.contains(standard().0),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
