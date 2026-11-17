//! The `carrying` service over a workspace that applies no rule, on values alone.

use super::service::carried;
use crate::cases::{Case, CaseName, Expect, Word, run};
use tastemaker_values::domain::applications::service::known;
use tastemaker_values::domain::definitions::fixtures::bare;
use tastemaker_values::domain::entries::fixtures::unwritten;
use tastemaker_values::domain::paths::types::RepoPath;
use tastemaker_values::domain::placement::service::place;
use tastemaker_values::domain::selectors::fixtures::refusing;

/// The files of the tree.
type Input = &'static [&'static str];
/// What the service answered, in words, empty when it is what the row expects.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_workspace_that_applies_no_rule_hands_none"),
    word: Word::Succeeds,
    input: || &["tastemaker.toml", "src/a/lib.txt"],
    expect: Expect::Value(String::new()),
}];

fn subject(files: Input) -> Value {
    let workspace = bare(files);
    let placement =
        place(&workspace, &refusing, &unwritten).expect("a workspace that composes nothing places");
    let said = match carried(
        &placement,
        &[],
        &known(&workspace, &placement),
        &refusing,
        &RepoPath::default(),
    ) {
        Ok(split) => format!(
            "{} handed, {} matched",
            split.handed.len(),
            split.matched.len()
        ),
        Err(failure) => format!("{failure:?}"),
    };
    if said == "0 handed, 0 matched" {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
