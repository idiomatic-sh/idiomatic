//! The pnpm adapter runs the manifest reading port's shared cases.
use super::adapter::ManifestReadingPnpmAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::manifest_reading::fixtures::contract::check;
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_pnpm_adapter_holds_the_project_contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| {
        check(&ManifestReadingPnpmAdapter::new(Vec::new()));
        Ok(())
    });
}
