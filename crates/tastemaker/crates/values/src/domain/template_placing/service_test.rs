//! The `template_placing` service's cases, on values alone.

use super::errors::TemplatePlacingError;
use super::service::{exclusion, names_parts};
use super::types::PartNaming;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::fixtures::bare;
use crate::domain::packages::types::DefinitionName;
use crate::domain::selectors::errors::SelectorsError;
use crate::domain::selectors::fixtures::refusing;
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
    /// The files a workspace's `rules-exclude` of `docs/**` covers, when the compiler refuses its glob.
    RefusedExclude,
}

/// What the row asks.
type Input = Question;
/// What the service answers, in words, empty when it is the one expected.
type Value = String;
type Failure = TemplatePlacingError;

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
    Case {
        name: CaseName("a_rules_exclude_the_compiler_refuses_is_refused_as_the_workspace_s"),
        word: Word::Refuses,
        input: || Question::RefusedExclude,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(TemplatePlacingError::Selector {
                    entry: "workspace".into(),
                    source: SelectorsError::Pattern {
                        pattern: "docs/**".into(),
                        reason: "refused".into(),
                    },
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// The answer, against the one the row's case expects: empty when they agree.
fn subject(question: Input) -> Result<Value, Failure> {
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
        Question::RefusedExclude => {
            let mut workspace = bare(&["docs/a.md", "src/lib.txt"]);
            workspace.section.rules_exclude =
                Selectors(vec![Selector::Glob(PathPattern("docs/**".into()))]);
            return exclusion(&workspace, &refusing, &[]).map(|excluded| format!("{excluded:?}"));
        }
    };
    let named = names_parts(&[&Selectors(vec![selector])]);
    Ok(if named == expected {
        String::new()
    } else {
        format!("{named:?}")
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
