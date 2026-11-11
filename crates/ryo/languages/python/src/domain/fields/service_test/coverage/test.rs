//! The check that every role of the vocabulary has a row in this file or in one of its parts.
use super::super::{
    CASES as ROLES, control_flow_roles_test, expression_roles_test,
    pattern_and_parameter_roles_test,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::labels::service::{field_name, fields};

/// Which tables a row reads.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The role tables of the concept and its three groups.
    EveryRoleTable,
}
/// What a row is given.
type Input = Given;
/// The roles of the vocabulary no row shows, joined by `,`.
type Value = &'static str;
/// Reading the tables is never refused.
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_role_of_the_vocabulary_has_a_row"),
    word: Word::Succeeds,
    input: || Given::EveryRoleTable,
    expect: Expect::Value(""),
}];

/// The roles no row of the tables shows.
fn subject(given: Input) -> Result<Value, Failure> {
    let Given::EveryRoleTable = given;
    let without_a_row: Vec<&str> = fields()
        .map(|field| field_name(field).0)
        .filter(|name| {
            !ROLES
                .iter()
                .chain(control_flow_roles_test::CASES)
                .chain(expression_roles_test::CASES)
                .chain(pattern_and_parameter_roles_test::CASES)
                .any(|case| matches!(case.expect, Expect::Value(wanted) if wanted == *name))
        })
        .collect();
    Ok(Box::leak(without_a_row.join(",").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
