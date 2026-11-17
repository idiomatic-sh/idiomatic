//! The placed-by kind selects the files one entry placed beneath the place.

use super::super::interface::SelectorsItem as _;
use super::item::PlacedByItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::captures::types::Interpolations;
use crate::domain::packages::types::DefinitionName;
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::fixtures::refusing;
use crate::domain::selectors::types::{
    PlacedBySelector, PlacedPath, Resolution, Resolved, SelectionContext, Selector,
};

/// The entry selected, and the place.
type Input = (&'static str, &'static str);
/// How many files it selects.
type Value = usize;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_files_the_entry_placed_beneath_the_place_are_selected"),
        word: Word::Succeeds,
        input: || ("errors", "core"),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_file_placed_elsewhere_is_not"),
        word: Word::Succeeds,
        input: || ("errors", "core/src/a"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_entry_that_placed_nothing_selects_nothing"),
        word: Word::Refuses,
        input: || ("types", "core"),
        expect: Expect::error(()),
    },
];

fn placed(path: &str, entry: &str) -> PlacedPath {
    PlacedPath {
        path: RepoPath(path.into()),
        entry: DefinitionName(entry.into()),
    }
}

fn subject((entry, place): Input) -> Result<Value, Failure> {
    let files = [
        placed("core/src/a/errors.txt", "errors"),
        placed("core/src/b/errors.txt", "errors"),
        placed("cli/src/errors.txt", "errors"),
    ];
    let base = RepoPath(place.into());
    let context = SelectionContext {
        base: &base,
        values: &Interpolations::default(),
        instances: &[],
        placed: &files,
        compile: &refusing,
    };
    let selector = Selector::PlacedBy(PlacedBySelector {
        entry: DefinitionName(entry.into()),
    });
    match PlacedByItem.resolve(&selector, &context) {
        Ok(Resolution::Resolved(Resolved::Paths(found))) if !found.is_empty() => Ok(found.len()),
        _ => Err(()),
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
