//! The `tokens` service's cases: an opening token's mirror closes its block.

use super::constants::STANDARD;
use super::service::mirror;
use super::types::Token;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// A token.
type Input = Token;
/// Its mirror.
type Value = Option<Token>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_function_closes_with_a_parenthesis"),
        word: Word::Succeeds,
        input: || Token::Function,
        expect: Expect::Value(Some(Token::RightParenthesis)),
    },
    Case {
        name: CaseName("a_curly_bracket_closes_with_its_own"),
        word: Word::Succeeds,
        input: || Token::LeftCurly,
        expect: Expect::Value(Some(Token::RightCurly)),
    },
    Case {
        name: CaseName("an_ident_opens_nothing"),
        word: Word::Succeeds,
        input: || Token::Ident,
        expect: Expect::Value(None),
    },
];

fn subject(token: Input) -> Value {
    mirror(token)
}

#[test]
fn succeeds() {
    assert!(STANDARD.contains("§4 Tokenization"));
    run(CASES, Word::Succeeds, |token| {
        Ok::<Value, Failure>(subject(token))
    });
}
