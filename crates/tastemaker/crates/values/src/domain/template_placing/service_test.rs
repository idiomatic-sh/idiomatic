//! The `template_placing` service's cases, on values alone.

use super::service::names_parts;
use super::types::PartNaming;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::packages::types::DefinitionName;
use crate::domain::selectors::types::{
    PartSelector, PathPattern, PlacedBySelector, Recursion, Selector, Selectors,
};

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// Whether a list of one glob names parts.
    Glob,
    /// Whether a list of one part selector names parts.
    Part,
    /// Whether a list of one placed-by selector names parts.
    PlacedBy,
}

/// What the row asks.
type Input = Question;
/// What the service answers, in words.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_glob_names_no_part"),
        word: Word::Succeeds,
        input: || Question::Glob,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_part_selector_names_the_instances_of_a_spec"),
        word: Word::Succeeds,
        input: || Question::Part,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_placed_by_selector_names_the_files_an_entry_placed"),
        word: Word::Succeeds,
        input: || Question::PlacedBy,
        expect: Expect::Value(String::new()),
    },
];

/// The answer, against the one the row's case expects: empty when they agree.
fn subject(question: Input) -> Value {
    let (selector, expected) = match question {
        Question::Part => (
            Selector::Part(PartSelector {
                part: DefinitionName("hex".into()),
                recursion: Recursion::Direct,
            }),
            PartNaming::NamesParts,
        ),
        Question::PlacedBy => (
            Selector::PlacedBy(PlacedBySelector {
                entry: DefinitionName("errors".into()),
            }),
            PartNaming::NamesParts,
        ),
        Question::Glob => (
            Selector::Glob(PathPattern("src/**".into())),
            PartNaming::NamesNone,
        ),
    };
    let named = names_parts(&[&Selectors(vec![selector])]);
    if named == expected {
        String::new()
    } else {
        format!("{named:?}")
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
