//! Typed cases for observed dotted project path membership.

use super::super::interface::RelationsItem as _;
use super::{
    constants,
    item::{ProjectNamespaceMembership, ProjectNamespaceMembershipRow},
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    relations::types::{Bits, CellKind, Class, FileId, Relation},
    terms::types::Symbol,
};

type Input = ();
type Value = [Bits; 5];
type Failure = ();

fn row() -> ProjectNamespaceMembershipRow {
    ProjectNamespaceMembershipRow {
        file: FileId(3),
        root: Symbol(5),
        path: Symbol(7),
        membership: Symbol(11),
        kind: Symbol(13),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("file_root_path_membership_kind_keep_five_distinct_cells"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(7), Bits(11), Bits(13)]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(ProjectNamespaceMembership::words(row()))
}

#[test]
fn row_words_keep_typed_column_order() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn relation_schema_exposes_all_observed_path_columns() {
    assert_eq!(ProjectNamespaceMembership::CLASS, Class::Manifest);
    assert_eq!(
        ProjectNamespaceMembership::relation(),
        Relation::ProjectNamespaceMembership
    );
    assert_eq!(
        ProjectNamespaceMembership::columns()
            .into_iter()
            .map(constants::column_name)
            .collect::<Vec<_>>(),
        ["file", "root", "path", "membership", "kind"]
    );
    assert_eq!(
        ProjectNamespaceMembership::kinds(),
        &[
            CellKind::File,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
        ]
    );
}
