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
        name: CaseName("a_token_is_named_as_the_kind_of_the_parser"),
        word: Word::Succeeds,
        input: || Question::Name(Token::LParen),
        expect: Expect::Value("l-paren"),
    },
    Case {
        name: CaseName("a_keyword_ends_in_kw"),
        word: Word::Succeeds,
        input: || Question::Name(Token::FnKw),
        expect: Expect::Value("fn-kw"),
    },
    Case {
        name: CaseName("the_names_come_from_the_pinned_parser"),
        word: Word::Succeeds,
        input: || Question::Standard,
        expect: Expect::Value("ra_ap_parser 0.0.347"),
    },
    Case {
        name: CaseName("there_are_156_tokens"),
        word: Word::Succeeds,
        input: || Question::Count,
        expect: Expect::Value("156"),
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
