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

/// What the row words and the relation schema answer.
#[derive(Debug, PartialEq, Eq)]
struct Observed {
    words: [Bits; 5],
    class: Class,
    relation: Relation,
    columns: [&'static str; 5],
    kinds: &'static [CellKind],
}

type Input = ();
type Value = Observed;
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
    expect: Expect::Value(Observed {
        words: [Bits(3), Bits(5), Bits(7), Bits(11), Bits(13)],
        class: Class::Manifest,
        relation: Relation::ProjectNamespaceMembership,
        columns: ["file", "root", "path", "membership", "kind"],
        kinds: &[
            CellKind::File,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
        ],
    }),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(Observed {
        words: ProjectNamespaceMembership::words(row()),
        class: ProjectNamespaceMembership::CLASS,
        relation: ProjectNamespaceMembership::relation(),
        columns: ProjectNamespaceMembership::columns()
            .into_iter()
            .map(constants::column_name)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_default(),
        kinds: ProjectNamespaceMembership::kinds(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
