//! The `reaching` service's cases, over the `check` fixture's workspace: the files its rule's entry reaches.

use super::service::reaches;
use super::types::{Selection, SelectorPair};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::fixtures::{applications, matching, placement, workspace};
use crate::domain::applications::service::known;

/// The fixture's tree.
const FILES: &[&str] = &[
    "tastemaker.toml",
    "src/a/errors.txt",
    "src/a/stray.txt",
    "src/b/lib.txt",
];

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The files the root's entry reaches.
    Reached,
    /// The root's entry, over a set compiler that refuses its patterns: the error names them, never an empty pattern.
    SetRefused,
}

type Input = Question;
/// What the row said, empty when it said what was expected.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_roots_entry_reaches_every_governed_file"),
        word: Word::Succeeds,
        input: || Question::Reached,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_set_that_does_not_compile_names_its_patterns"),
        word: Word::Succeeds,
        input: || Question::SetRefused,
        expect: Expect::Value(String::new()),
    },
];

/// A set compiler that refuses every set, as one whose pattern does not compile does.
fn refusing(
    _globs: &[crate::domain::selectors::types::SetGlob],
) -> Result<
    std::sync::Arc<dyn crate::domain::selectors::types::PathSet>,
    crate::domain::selectors::types::PatternFailure,
> {
    Err(crate::domain::selectors::types::PatternFailure(
        "unclosed class".into(),
    ))
}

fn subject(question: Input) -> Value {
    let workspace = workspace(FILES);
    let placement = placement(&workspace);
    let applied = applications(&workspace, &placement);
    if let Question::SetRefused = question {
        let refusing_matching = crate::domain::applications::types::Compilers {
            compile: matching().compile,
            sets: &refusing,
        };
        let reached = reaches(
            &workspace,
            &placement,
            &applied,
            &known(&workspace, &placement),
            refusing_matching,
        );
        return match reached.values().next() {
            Some(Err(crate::domain::placement::errors::PlacementError::Selector {
                source: crate::domain::selectors::errors::SelectorsError::Pattern { pattern, .. },
                ..
            })) if !pattern.is_empty() => String::new(),
            other => format!("{other:?}"),
        };
    }
    let (said, expected) = match question {
        Question::Reached => {
            let reached = reaches(
                &workspace,
                &placement,
                &applied,
                &known(&workspace, &placement),
                matching(),
            );
            let entry = applied[0].entry;
            let key = Selection {
                scope: applied[0].scope,
                pair: SelectorPair {
                    include: &entry.header.include,
                    exclude: &entry.header.exclude,
                },
            };
            let said = match reached.get(&key) {
                Some(Ok(files)) => files
                    .iter()
                    .map(|file| &*file.0)
                    .collect::<Vec<_>>()
                    .join(","),
                Some(Err(failure)) => format!("{failure:?}"),
                None => "no answer".to_owned(),
            };
            (
                said,
                "tastemaker.toml,src/a/errors.txt,src/a/stray.txt,src/b/lib.txt".to_owned(),
            )
        }
        Question::SetRefused => (String::new(), String::new()),
    };
    if said == expected {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
