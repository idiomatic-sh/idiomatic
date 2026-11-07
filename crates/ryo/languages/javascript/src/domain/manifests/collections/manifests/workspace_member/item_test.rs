//! Workspace package glob selection excludes named nonmembers.

use super::item::WorkspaceMemberItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{WORKSPACE, read};

type Input = &'static [(&'static str, &'static str)];
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("only_matching_workspace_packages_are_members"),
    word: Word::Succeeds,
    input: || WORKSPACE,
    expect: Expect::Value(true),
}];
fn subject(files: Input) -> Value {
    let (written, _) = read::<WorkspaceMemberItem>(files);
    written.partial == 0
        && written
            .rows
            .iter()
            .map(|row| (row.workspace.0, row.package.0))
            .collect::<Vec<_>>()
            == [(1, 3), (1, 5)]
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
