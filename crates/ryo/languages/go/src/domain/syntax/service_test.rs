//! The `syntax` service's cases.

use super::service::{kind_bits, kind_name, kind_of, kinds};
use super::types::Kind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::tokens::types::Token;
use std::collections::BTreeSet;

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// Whether every kind's number names it again and every kind has a name, and how many kinds there are.
    Every,
    /// The name of an extra.
    Extra(Extra),
    /// How many extras there are.
    Extras,
    /// The name of a kind.
    Named(Kind),
    /// Whether the names of the kinds are all different.
    Distinct,
    /// Whether the numbers of the kinds are their places.
    Places,
    /// The kind of a number no kind has.
    Unnumbered,
    /// Whether the kinds are named as `go/ast` and `go/token` name them, in kebab case.
    Standard,
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
        expect: Expect::Value("true 147"),
    },
    Case {
        name: CaseName("text_the_parser_could_not_read_is_an_error"),
        word: Word::Succeeds,
        input: || Question::Extra(Extra::Error),
        expect: Expect::Value("error"),
    },
    Case {
        name: CaseName("white_space_is_an_extra"),
        word: Word::Succeeds,
        input: || Question::Extra(Extra::Whitespace),
        expect: Expect::Value("whitespace"),
    },
    Case {
        name: CaseName("there_are_nine_extras_the_two_of_a_go_source_and_the_seven_of_a_go_mod"),
        word: Word::Succeeds,
        input: || Question::Extras,
        expect: Expect::Value("9"),
    },
    Case {
        name: CaseName("a_token_that_is_also_a_node_ends_in_token"),
        word: Word::Succeeds,
        input: || Question::Named(Kind::Token(Token::Ident)),
        expect: Expect::Value("ident-token"),
    },
    Case {
        name: CaseName("the_package_keyword_is_not_the_package_node"),
        word: Word::Succeeds,
        input: || Question::Named(Kind::Token(Token::Package)),
        expect: Expect::Value("package-token"),
    },
    Case {
        name: CaseName("the_names_of_the_kinds_are_all_different"),
        word: Word::Succeeds,
        input: || Question::Distinct,
        expect: Expect::Value("true"),
    },
    Case {
        name: CaseName("the_numbers_of_the_kinds_are_their_places"),
        word: Word::Succeeds,
        input: || Question::Places,
        expect: Expect::Value("true"),
    },
    Case {
        name: CaseName("a_number_no_kind_has_is_an_illegal_token"),
        word: Word::Succeeds,
        input: || Question::Unnumbered,
        expect: Expect::Value("illegal"),
    },
    Case {
        name: CaseName("the_names_are_those_of_go_ast_and_go_token_in_kebab_case"),
        word: Word::Succeeds,
        input: || Question::Standard,
        expect: Expect::Value("true"),
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
        Question::Named(kind) => kind_name(kind).0.to_owned(),
        Question::Distinct => {
            let names: BTreeSet<&str> = kinds().map(|kind| kind_name(kind).0).collect();
            (names.len() == kinds().count()).to_string()
        }
        Question::Places => {
            let numbers: Vec<u16> = kinds().map(|kind| kind_bits(kind).0).collect();
            let expected: Vec<u16> = (0..u16::try_from(numbers.len()).unwrap_or(0)).collect();
            (numbers == expected).to_string()
        }
        Question::Unnumbered => kind_name(kind_of(ryo::languages::KindBits(u16::MAX)))
            .0
            .to_owned(),
        Question::Standard => {
            let names: BTreeSet<&str> = kinds().map(|kind| kind_name(kind).0).collect();
            [
                "func-decl",
                "index-list-expr",
                "type-switch-stmt",
                "and-not-assign",
                "lparen",
                "whitespace",
            ]
            .iter()
            .all(|expected| names.contains(expected))
            .to_string()
        }
    };
    Box::leak(answer.into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<Value, Failure>(subject(input))
    });
}
