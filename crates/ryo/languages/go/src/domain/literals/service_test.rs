//! The `literals` service's cases: a string literal is the text it stands for, as `strconv.Unquote` reads it.

use super::service::unquoted;
use super::types::Written;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// A string literal as a file writes it.
type Input = &'static str;
/// The text it stands for; nothing for a literal that is no text.
type Value = Option<&'static str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_string_with_no_escape_is_its_content"),
        word: Word::Succeeds,
        input: || "\"plain é\"",
        expect: Expect::Value(Some("plain é")),
    },
    Case {
        name: CaseName("the_escapes_of_an_interpreted_string_are_resolved"),
        word: Word::Succeeds,
        input: || "\"a\\tb\\n\\\\ \\\" \\x41 \\101 \\u00e9 \\U0001F600\"",
        expect: Expect::Value(Some("a\tb\n\\ \" A A é 😀")),
    },
    Case {
        name: CaseName("a_raw_string_keeps_its_backslashes_and_drops_its_carriage_returns"),
        word: Word::Succeeds,
        input: || "`a\\n\r\nb`",
        expect: Expect::Value(Some("a\\n\nb")),
    },
    Case {
        name: CaseName("an_escape_that_makes_a_byte_that_is_no_utf_8_is_no_text"),
        word: Word::Succeeds,
        input: || "\"\\xff\"",
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("an_escape_go_does_not_have_is_no_text"),
        word: Word::Succeeds,
        input: || "\"\\q \\' \\400 \\ud800\"",
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("a_literal_that_is_not_closed_is_no_text"),
        word: Word::Succeeds,
        input: || "\"open",
        expect: Expect::Value(None),
    },
];

fn subject(literal: Input) -> Value {
    unquoted(Written(literal)).map(|text| &*Box::leak(text.0.into_owned().into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<Value, Failure>(subject(input))
    });
}
