//! The `syntax` service's cases.

use super::service::{
    field_bits, field_name, fields, kind_bits, kind_name, kind_of, kinds, value_kind,
};
use super::types::Kind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// Whether every kind's number names it again and every kind has a name, and how many kinds there are.
    Every,
    /// The name of an extra.
    Extra(Extra),
    /// How many extras there are.
    Extras,
    /// The names of the roles, in the order of their numbers, each with its number.
    Roles,
    /// What a kind of element is as a value.
    Value(Kind),
}

/// What a row asks.
type Input = Question;
/// The answer, as text.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_kind_is_found_by_its_number_and_has_a_name"),
        word: Word::Succeeds,
        input: || Question::Every,
        expect: Expect::Value("true 32"),
    },
    Case {
        name: CaseName("text_the_parser_could_not_read_is_an_error"),
        word: Word::Succeeds,
        input: || Question::Extra(Extra::Error),
        expect: Expect::Value("error"),
    },
    Case {
        name: CaseName("there_is_one_extra"),
        word: Word::Succeeds,
        input: || Question::Extras,
        expect: Expect::Value("1"),
    },
    Case {
        name: CaseName("every_role_is_named_by_its_number"),
        word: Word::Succeeds,
        input: || Question::Roles,
        expect: Expect::Value("0:none 1:key 2:val 3:simple-key"),
    },
    Case {
        name: CaseName("every_spelling_of_a_string_is_a_string"),
        word: Word::Succeeds,
        input: || Question::Value(Kind::Token(Token::MlLiteralString)),
        expect: Expect::Value("Some(String)"),
    },
    Case {
        name: CaseName("an_inline_table_is_an_inline_mapping"),
        word: Word::Succeeds,
        input: || Question::Value(Kind::Node(Node::InlineTable)),
        expect: Expect::Value("Some(InlineMapping)"),
    },
    Case {
        name: CaseName("text_the_parser_could_not_read_is_no_value"),
        word: Word::Succeeds,
        input: || Question::Value(Kind::Extra(Extra::Error)),
        expect: Expect::Value("None"),
    },
];

fn subject(question: Input) -> Value {
    let answer = match question {
        Question::Every => {
            let round = kinds()
                .all(|kind| kind_of(kind_bits(kind)) == kind && !kind_name(kind).0.is_empty());
            format!("{round} {}", kinds().count())
        }
        Question::Extra(extra) => kind_name(Kind::Extra(extra)).0.to_owned(),
        Question::Extras => kinds()
            .filter(|kind| matches!(kind, Kind::Extra(_)))
            .count()
            .to_string(),
        Question::Roles => fields()
            .map(|field| format!("{}:{}", field_bits(field).0, field_name(field).0))
            .collect::<Vec<_>>()
            .join(" "),
        Question::Value(kind) => format!("{:?}", value_kind(kind)),
    };
    Box::leak(answer.into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<Value, Failure>(subject(input))
    });
}
