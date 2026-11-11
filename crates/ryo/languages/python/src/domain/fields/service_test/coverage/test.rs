//! The check that every role of the vocabulary has a row in this file or in one of its parts.
use super::super::{
    CASES, control_flow_roles_test, expression_roles_test, pattern_and_parameter_roles_test,
};
use crate::cases::Expect;
use crate::domain::labels::service::{field_name, fields};

#[test]
/// The roles of the vocabulary no row shows.
fn every_role_of_the_vocabulary_has_a_row() {
    let without_a_row: Vec<_> = fields()
        .filter(|field| {
            let name = field_name(*field).0;
            !CASES
                .iter()
                .chain(control_flow_roles_test::CASES)
                .chain(expression_roles_test::CASES)
                .chain(pattern_and_parameter_roles_test::CASES)
                .any(|case| matches!(case.expect, Expect::Value(wanted) if wanted == name))
        })
        .collect();
    assert_eq!(without_a_row, Vec::new());
}
