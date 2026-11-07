//! The `tokens` service's cases.

use super::service::standard;
use super::types::Token;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::kind_name;
use crate::domain::syntax::types::Kind;
use strum::EnumCount as _;

/// What a row asks: a token's name, how many tokens there are, or the standard.
type Input = Question;
/// The answer, as text.
type Value = &'static str;
type Failure = ();

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The name of a token, as the standard names it.
    Name(Token),
    /// How many tokens there are.
    Count,
    /// The standard the names come from.
    Standard,
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("token_is_named_as_the_standard_names_it"),
        word: Word::Succeeds,
        input: || Question::Name(Token::Identifier),
        expect: Expect::Value("identifier-token"),
    },
    Case {
        name: CaseName("the_names_come_from_the_standard"),
        word: Word::Succeeds,
        input: || Question::Standard,
        expect: Expect::Value("ECMAScript lexical token classes; Acorn 8.15.0 tokens and comments"),
    },
    Case {
        name: CaseName("the_language_has_one_token_so_far"),
        word: Word::Succeeds,
        input: || Question::Count,
        expect: Expect::Value("12"),
    },
];

fn subject(question: Input) -> Value {
    match question {
        Question::Name(each) => kind_name(Kind::Token(each)).0,
        Question::Count => Box::leak(Token::COUNT.to_string().into_boxed_str()),
        Question::Standard => standard().0,
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
