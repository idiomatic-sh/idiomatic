//! The regex kind covers the paths its expression matches beneath the place, and what lies beneath them.

use std::sync::Arc;

use super::super::interface::SelectorsItem as _;
use super::item::RegexItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::captures::types::Interpolations;
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::service::covers_any;
use crate::domain::selectors::types::{
    PathTest, PatternFailure, RegexText, Resolution, ResolvedSelectors, SelectionContext, Selector,
    Spelling,
};
use crate::outbound::matching::factory::globset;
use crate::outbound::matching::interface::MatchingPort as _;

/// The expression, and the path asked about.
type Input = (&'static str, &'static str);
/// Whether it is covered.
type Value = bool;
type Failure = ();

fn compile(spelling: &Spelling) -> Result<Arc<dyn PathTest>, PatternFailure> {
    globset().compile(spelling)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_path_the_expression_matches_is_covered"),
        word: Word::Succeeds,
        input: || (r"^src/.*_test\.txt$", "crate/src/a_test.txt"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_path_it_does_not_match_is_not"),
        word: Word::Succeeds,
        input: || (r"^src/.*_test\.txt$", "crate/src/a.txt"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("an_expression_that_does_not_compile_is_refused"),
        word: Word::Refuses,
        input: || ("(", "crate/src/a.txt"),
        expect: Expect::error(()),
    },
];

fn subject((expression, path): Input) -> Result<Value, Failure> {
    let base = RepoPath("crate".into());
    let context = SelectionContext {
        base: &base,
        values: &Interpolations::default(),
        instances: &[],
        placed: &[],
        compile: &compile,
    };
    let Resolution::Resolved(resolved) = RegexItem
        .resolve(&Selector::Regex(RegexText(expression.into())), &context)
        .map_err(drop)?
    else {
        return Err(());
    };
    Ok(
        covers_any(&ResolvedSelectors(vec![resolved]), &RepoPath(path.into()))
            == crate::domain::selectors::types::Coverage::Covered,
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
