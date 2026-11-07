//! An undeclared project inventory is complete only within nothing, and lists no entries.
use crate::cases::{Case, CaseName, Expect, Word, run};
use super::value::{InventoryCompleteness, ProjectInventoryDocument};

type Input = ();
type Value = (InventoryCompleteness, usize);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a default inventory is partial and empty"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((InventoryCompleteness::Partial, 0)),
}];
fn subject((): Input) -> Result<Value, Failure> {
    let inventory = ProjectInventoryDocument::default();
    Ok((inventory.completeness, inventory.entries.len()))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
