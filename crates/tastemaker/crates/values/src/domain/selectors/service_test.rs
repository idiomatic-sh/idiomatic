//! What the `selectors` service refuses of a pattern's spelling, before any is compiled.

use super::errors::SelectorsError;
use super::service::{place, resolve};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::captures::types::Interpolations;
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::fixtures::refusing;
use crate::domain::selectors::types::{PathPattern, SelectionContext, Selectors};

/// What a row places or resolves.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// A pattern, placed at the root.
    Placed(&'static str),
    /// A list of no selector, resolved at the root.
    Empty,
}

type Input = Asked;
/// What placing it answers.
type Value = String;
type Failure = SelectorsError;

/// Whether `error` refuses the pattern for the one reason a capture's mark is refused for.
fn a_misplaced_mark(error: &SelectorsError) -> Selection {
    match error {
        SelectorsError::Pattern { reason, .. }
            if reason
                == "a capture's name holds no `:`; the one mark is `:dir`, written `{name:dir}`" =>
        {
            Selection::Matches
        }
        SelectorsError::Pattern { .. } | SelectorsError::Unresolved { .. } => Selection::Differs,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_list_of_no_selector_resolves_to_nothing"),
        word: Word::Succeeds,
        input: || Asked::Empty,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_span_capture_with_a_mark_is_refused"),
        word: Word::Refuses,
        input: || Asked::Placed("{*name:dir}/x.txt"),
        expect: Expect::Refusal(a_misplaced_mark),
    },
    Case {
        name: CaseName("a_capture_with_a_misspelt_mark_is_refused"),
        word: Word::Refuses,
        input: || Asked::Placed("{name:dirs}.txt"),
        expect: Expect::Refusal(a_misplaced_mark),
    },
];

fn subject(asked: Input) -> Result<Value, Failure> {
    let base = RepoPath(String::new().into());
    let values = Interpolations::default();
    match asked {
        Asked::Placed(pattern) => place(&base, &PathPattern(pattern.into()), &values, &refusing)
            .map(|placed| format!("{placed:?}")),
        Asked::Empty => {
            let context = SelectionContext {
                base: &base,
                values: &values,
                instances: &[],
                placed: &[],
                compile: &refusing,
            };
            let resolved = resolve(&Selectors::default(), &context)?;
            Ok(if resolved.0.is_empty() {
                String::new()
            } else {
                format!("{resolved:?}")
            })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
