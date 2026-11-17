//! The `planning` service over a workspace that applies no rule, on values alone.

use super::service::plan;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::fixtures::bare;
use crate::domain::entries::fixtures::unwritten;
use crate::domain::paths::types::RepoPath;
use crate::domain::placement::service::place;
use crate::domain::selectors::fixtures::refusing;

/// The files of the tree.
type Input = &'static [&'static str];
/// What the service answered, in words, empty when it is what the row expects.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_workspace_that_applies_no_rule_plans_only_the_built_in_rules"),
    word: Word::Succeeds,
    input: || &["tastemaker.toml", "src/a/lib.txt"],
    expect: Expect::Value(String::new()),
}];

fn subject(files: Input) -> Value {
    let workspace = bare(files);
    let placement =
        place(&workspace, &refusing, &unwritten).expect("a workspace that composes nothing places");
    let said = match plan(
        &workspace,
        &placement,
        RepoPath::default(),
        crate::domain::applications::types::Compilers {
            compile: &refusing,
            sets: &crate::domain::selectors::fixtures::refusing_sets,
        },
    ) {
        Ok(planned) => planned
            .rules
            .iter()
            .map(|rule| &*rule.id.0)
            .collect::<Vec<_>>()
            .join(","),
        Err(failure) => format!("{failure:?}"),
    };
    if said == "tastemaker-comments" {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
