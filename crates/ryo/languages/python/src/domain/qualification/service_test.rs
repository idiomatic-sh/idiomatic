//! The `qualification` service's cases: the builtin catalog answers for builtins only.
use super::service::builtin_named;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::languages::NameText;

/// A name.
type Input = &'static str;
/// Whether the catalog knows it.
type Value = bool;
/// Nothing is refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case { name: CaseName("a_builtin_is_in_the_catalog"), word: Word::Succeeds, input: || "exec", expect: Expect::Value(true) },
    Case { name: CaseName("another_name_is_not"), word: Word::Succeeds, input: || "not_a_builtin_name", expect: Expect::Value(false) },
];

fn subject(name: Input) -> Result<Value, Failure> {
    Ok(builtin_named(NameText(name)).is_some())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
