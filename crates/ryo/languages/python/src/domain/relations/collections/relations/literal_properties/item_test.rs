//! Preserved validation diagnostics do not make literal rows incomplete.

use crate::cases::{Case, CaseName, Expect, Word, run};
use super::super::fixtures::contract::check;
use super::item::LiteralPropertiesItem;
use crate::fixtures::reading::reading;

fn module_nonlocal_validation_keeps_literal_properties_complete() {
    check::<LiteralPropertiesItem>();
    let result = reading::<LiteralPropertiesItem>("nonlocal x\n1\n");
    assert!(result.causes.is_empty());
    assert_eq!(result.rows.len(), 1);
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("module_nonlocal_validation_keeps_literal_properties_complete"),
        word: Word::Succeeds,
        input: || module_nonlocal_validation_keeps_literal_properties_complete,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}
