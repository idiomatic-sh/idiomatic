//! What placement finds over a workspace that composes nothing, on values alone.

use super::errors::PlacementError;
use super::service::place;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::fixtures::bare;
use crate::domain::entries::fixtures::unwritten;
use crate::domain::findings::service::printed;
use crate::domain::selectors::fixtures::refusing;

/// The files of the tree.
type Input = &'static [&'static str];
/// The scopes, the placed files and the findings, in words, empty when they are what the row expects.
type Value = String;
type Failure = PlacementError;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_workspace_that_composes_nothing_is_one_scope_that_places_nothing"),
    word: Word::Succeeds,
    input: || &["tastemaker.toml", "src/a/lib.txt"],
    expect: Expect::Value(String::new()),
}];

fn subject(files: Input) -> Result<Value, Failure> {
    let placement = place(&bare(files), &refusing, &unwritten)?;
    let said = format!(
        "{} scopes, {} placed, findings: {}",
        placement.scopes.len(),
        placement.placed.len(),
        placement
            .findings
            .iter()
            .map(|finding| printed(&finding.rule).0.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(if said == "1 scopes, 0 placed, findings: " {
        String::new()
    } else {
        said
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
