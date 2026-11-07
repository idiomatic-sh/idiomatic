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
        name: CaseName("a_token_is_named_as_the_rule_of_the_abnf"),
        word: Word::Succeeds,
        input: || Question::Name(Token::KeyvalSep),
        expect: Expect::Value("keyval-sep"),
    },
    Case {
        name: CaseName("white_space_is_ws"),
        word: Word::Succeeds,
        input: || Question::Name(Token::Ws),
        expect: Expect::Value("ws"),
    },
    Case {
        name: CaseName("the_names_come_from_the_abnf_of_toml_1_1"),
        word: Word::Succeeds,
        input: || Question::Standard,
        expect: Expect::Value("toml.abnf 1.1.0"),
    },
    Case {
        name: CaseName("there_are_twenty_four_tokens"),
        word: Word::Succeeds,
        input: || Question::Count,
        expect: Expect::Value("24"),
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
