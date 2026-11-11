//! The check that every role of the vocabulary has a row in this file or in one of its parts.

use super::{control_flow_roles_test, expression_roles_test, pattern_and_parameter_roles_test};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::labels::service::{field_name, fields};

/// The roles of the vocabulary no row shows.
fn every_role_of_the_vocabulary_has_a_row() {
    let without_a_row: Vec<_> = fields()
        .filter(|field| {
            let name = field_name(*field).0;
            !super::CASES
                .iter()
                .chain(control_flow_roles_test::CASES)
                .chain(expression_roles_test::CASES)
                .chain(pattern_and_parameter_roles_test::CASES)
                .any(|case| matches!(case.expect, Expect::Value(wanted) if wanted == name))
        })
        .collect();
    assert_eq!(without_a_row, Vec::new());
}

/// Each check below states its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[Case {
    name: CaseName("every_role_of_the_vocabulary_has_a_row"),
    word: Word::Succeeds,
    input: || every_role_of_the_vocabulary_has_a_row,
    expect: Expect::Value(()),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}
