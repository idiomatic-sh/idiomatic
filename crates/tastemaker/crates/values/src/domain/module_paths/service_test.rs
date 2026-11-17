//! What the `module_paths` service reads of a module path and an `imports` item, as the matching port reads a path.

use super::service::{pattern, prefixes};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::imports::types::{AllowItem, WrittenPath};

/// What a row reads.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The modules a written path lies in.
    Prefixes(&'static str),
    /// An item's module pattern.
    Pattern(&'static str),
}

type Input = Question;
/// What was read, joined by spaces.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_written_path_lies_in_each_module_above_it"),
        word: Word::Succeeds,
        input: || Question::Prefixes("a::b::c"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_written_with_dots_lies_in_each_module_above_it"),
        word: Word::Succeeds,
        input: || Question::Prefixes("a.b.c"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_written_with_slashes_keeps_a_scope_in_its_first_name"),
        word: Word::Succeeds,
        input: || Question::Prefixes("@a/b-c/d"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_star_segment_stands_for_any_number_of_segments"),
        word: Word::Succeeds,
        input: || Question::Pattern("*::interface"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_star_within_a_segment_stays_one_segment"),
        word: Word::Succeeds,
        input: || Question::Pattern("ryo_*"),
        expect: Expect::Value(String::new()),
    },
];

/// What the service reads each question as.
fn expected(question: Question) -> &'static str {
    match question {
        Question::Prefixes("@a/b-c/d") => "@a @a/b-c @a/b-c/d",
        Question::Prefixes(_) => "a a/b a/b/c",
        Question::Pattern("*::interface") => "**/interface",
        Question::Pattern(_) => "ryo_*",
    }
}

fn subject(question: Input) -> Value {
    let said = match question {
        Question::Prefixes(written) => prefixes(&WrittenPath::from(written))
            .iter()
            .map(|prefix| prefix.0.to_string())
            .collect::<Vec<_>>()
            .join(" "),
        Question::Pattern(item) => pattern(&AllowItem::from(item)).0.into_string(),
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
