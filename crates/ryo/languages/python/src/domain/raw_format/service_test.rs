//! The `raw_format` service's cases: the body of a raw token.
use super::service::body;
use super::types::RawFormatToken;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// A raw string token.
type Input = &'static str;
/// The body, as text.
type Value = &'static str;
/// Nothing is refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_bytes_token_has_no_format_body"),
        word: Word::Succeeds,
        input: || "rb'%d'",
        expect: Expect::Value("None"),
    },
    Case {
        name: CaseName("a_triple_quoted_token_has_its_inner_body"),
        word: Word::Succeeds,
        input: || "'''%s'''",
        expect: Expect::Value("Some(\"%s\")"),
    },
];

fn subject(token: Input) -> Result<Value, Failure> {
    Ok(Box::leak(format!("{:?}", body(RawFormatToken(token)).map(|each| each.0)).into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
