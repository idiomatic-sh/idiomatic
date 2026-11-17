//! What the `selectors` service refuses of a pattern's spelling, before any is compiled.

use super::errors::SelectorsError;
use super::service::place;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::captures::types::Interpolations;
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::fixtures::refusing;
use crate::domain::selectors::types::{Coverage, PathPattern};

/// The pattern placed.
type Input = &'static str;
/// Why placing it is refused, empty when it is the one reason a capture's mark is refused for.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_span_capture_with_a_mark_is_refused"),
        word: Word::Succeeds,
        input: || "{*name:dir}/x.txt",
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_capture_with_a_misspelt_mark_is_refused"),
        word: Word::Succeeds,
        input: || "{name:dirs}.txt",
        expect: Expect::Value(String::new()),
    },
];

fn subject(pattern: Input) -> Result<Value, Failure> {
    let placed = place(
        &RepoPath(String::new().into()),
        &PathPattern(pattern.into()),
        &Interpolations::default(),
        &refusing,
    );
    Ok(match placed {
        Err(SelectorsError::Pattern { reason, .. })
            if reason
                == "a capture's name holds no `:`; the one mark is `:dir`, written `{name:dir}`" =>
        {
            String::new()
        }
        Err(SelectorsError::Pattern { reason, .. }) => reason,
        other => format!("{other:?}"),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    assert_eq!(Coverage::from(true), Coverage::Covered);
}
