//! What the `captures` service makes of what a pattern's captures took: text with them replaced, the captures kept, and
//! the captures as a message writes them.

use std::collections::BTreeSet;

use super::service::{
    captured_names, captures_text, escaped, inheriting, project, substitute, substitute_pattern,
    unescaped, wildcards,
};
use super::types::{
    CaptureBinding, CaptureName, CaptureValue, Captures, PlainText, Template, Wildcards,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row asks of the captures `a = 1, b = 2`.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// This text with the captures replaced.
    Substitute(&'static str),
    /// The captures named `a` only.
    Project,
    /// The captures beside `b = 3, c = 4`, theirs giving way.
    Inheriting,
    /// This text written into a path pattern as itself.
    Escaped(&'static str),
    /// The path this pattern names, its escapes taken away.
    Unescaped(&'static str),
    /// This pattern with the captures put in place, still a pattern.
    SubstitutePattern(&'static str),
    /// Whether this pattern holds a wildcard, and the names it captures.
    Reads(&'static str),
}

type Input = Question;
/// The answer, as a message writes captures, or the text replaced.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_and_a_span_name_are_replaced_by_their_values"),
        word: Word::Succeeds,
        input: || Question::Substitute("src/{a}/{*b}/{c}"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("only_the_captures_named_are_kept"),
        word: Word::Succeeds,
        input: || Question::Project,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_capture_of_its_own_wins_over_the_enclosing_one"),
        word: Word::Succeeds,
        input: || Question::Inheriting,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_name_with_the_patterns_characters_is_written_with_each_escaped"),
        word: Word::Succeeds,
        input: || Question::Escaped("a*b?{c}\\[1]"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_escaped_character_names_itself_in_the_path"),
        word: Word::Succeeds,
        input: || Question::Unescaped("data[1]/\\*\\{x\\}"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_escaped_brace_is_text_and_a_capture_beside_it_is_put_in_place"),
        word: Word::Succeeds,
        input: || Question::SubstitutePattern("\\{a\\}/{a}/{*b}/{c}"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_escaped_wildcard_or_brace_is_no_wildcard_and_no_capture"),
        word: Word::Succeeds,
        input: || Question::Reads("\\*\\?\\{a\\}{b}"),
        expect: Expect::Value(String::new()),
    },
];

fn captures(pairs: &[(&str, &str)]) -> Captures {
    Captures(
        pairs
            .iter()
            .map(|(name, value)| (CaptureName((*name).into()), CaptureValue((*value).into())))
            .collect(),
    )
}

/// What each question is answered with when the service is right.
fn expected(question: Question) -> &'static str {
    match question {
        Question::Substitute(_) => "src/1/2/{c}",
        Question::Project => "a = 1",
        Question::Inheriting => "a = 1, b = 2, c = 4",
        Question::Escaped(_) => "a\\*b\\?\\{c\\}\\\\[1]",
        Question::Unescaped(_) => "data[1]/*{x}",
        Question::SubstitutePattern(_) => "\\{a\\}/1/2/{c}",
        Question::Reads(_) => "false b",
    }
}

fn subject(question: Input) -> Value {
    let own = captures(&[("a", "1"), ("b", "2")]);
    let said = match question {
        Question::Substitute(text) => {
            substitute(
                Template(text),
                own.0
                    .iter()
                    .map(|(name, value)| CaptureBinding { name, value }),
            )
            .0
        }
        Question::Project => {
            captures_text(&project(&own, &BTreeSet::from([CaptureName("a".into())]))).0
        }
        Question::Inheriting => {
            captures_text(&inheriting(&own, &captures(&[("b", "3"), ("c", "4")]))).0
        }
        Question::Escaped(text) => escaped(PlainText(text)).0,
        Question::Unescaped(text) => unescaped(Template(text)).0,
        Question::SubstitutePattern(text) => {
            substitute_pattern(
                Template(text),
                own.0
                    .iter()
                    .map(|(name, value)| CaptureBinding { name, value }),
            )
            .0
        }
        Question::Reads(text) => format!(
            "{} {}",
            wildcards(Template(text)) == Wildcards::Held,
            captured_names(Template(text))
                .iter()
                .map(|name| &*name.0)
                .collect::<Vec<_>>()
                .join(" ")
        ),
    };
    if said == expected(question) {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
