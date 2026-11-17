//! The regex kind resolves only a regex selector, and names its expression when the compiler refuses it.

use super::super::interface::SelectorsItem as _;
use super::item::RegexItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::captures::types::Interpolations;
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::errors::SelectorsError;
use crate::domain::selectors::fixtures::refusing;
use crate::domain::selectors::types::{
    PathPattern, RegexText, Resolution, SelectionContext, Selector,
};

/// The selector resolved.
type Input = Selector;
/// Whether it is another kind's.
type Value = bool;
type Failure = SelectorsError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_selector_of_another_kind_is_not_mine"),
        word: Word::Succeeds,
        input: || Selector::Glob(PathPattern("src/**".into())),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_expression_the_compiler_refuses_is_refused_by_its_text"),
        word: Word::Refuses,
        input: || Selector::Regex(RegexText("(".into())),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(SelectorsError::Pattern {
                    pattern: "(".into(),
                    reason: "refused".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

fn subject(selector: Input) -> Result<Value, Failure> {
    let base = RepoPath("crate".into());
    let context = SelectionContext {
        base: &base,
        values: &Interpolations::default(),
        instances: &[],
        placed: &[],
        compile: &refusing,
    };
    Ok(matches!(
        RegexItem.resolve(&selector, &context)?,
        Resolution::NotMine
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
