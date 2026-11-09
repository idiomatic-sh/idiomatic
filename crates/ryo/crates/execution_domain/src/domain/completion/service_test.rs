//! The `completion` service's cases: what a coverage lacks, and the file a node of a partial lies in.

use super::service::{lacks_of, partial_any, partial_of_lacks};
use super::types::{CoverageState, FileLacks, MissingRows};
use crate::domain::coverage::types::LackingKind;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// What a row of the cases proves.
#[derive(Clone, Copy)]
enum Scenario {
    /// How many files a coverage partial in some files names, one of them twice.
    FilesNamed,
    /// How many files that lack any row the lacks of a file nobody said anything of name.
    UnnamedFileLacksAnyRow,
    /// How many kinds are missing from a file two things said different kinds of.
    KindsUnited,
    /// How many relations with a node column have not exactly one file column.
    NodeColumnsWithoutAFile,
}

/// What a row of the cases asks.
type Input = Scenario;
/// A count.
type Value = usize;
type Failure = ();

/// What the catalog lets a partial's file place: a node lies in the file its row names, so every relation with a node column has a
/// file column to lie in. (A node column that points into another file would escape the file a partial scopes to; the catalog does
/// not say which file a node column is of, so only this shape is asserted and the rest is the relation's own contract.)
fn node_columns_without_a_file() -> Value {
    use ryo_relations::domain::relations::{
        service::{all, columns, kind},
        types::CellKind,
    };
    all()
        .iter()
        .filter(|relation| {
            let cells: Vec<CellKind> = columns(**relation).into_iter().map(kind).collect();
            cells.contains(&CellKind::Node)
                && cells.iter().filter(|each| **each == CellKind::File).count() != 1
        })
        .count()
}

fn kinds_missing(kinds: &[&str]) -> FileLacks {
    FileLacks {
        file: Some(FileId(1)),
        missing: MissingRows::Kinds {
            column: ColumnIndex(1),
            kinds: kinds
                .iter()
                .map(|kind| LackingKind((*kind).into()))
                .collect(),
        },
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(match input {
        Scenario::FilesNamed => {
            let CoverageState::Partial { files, .. } =
                partial_any(vec![Some(FileId(1)), None, Some(FileId(1))])
            else {
                return Err(());
            };
            files.len()
        }
        Scenario::UnnamedFileLacksAnyRow => lacks_of(&[Some(FileId(1))], &[])
            .iter()
            .filter(|each| each.missing == MissingRows::Any)
            .count(),
        Scenario::KindsUnited => {
            let CoverageState::Partial { lacks, .. } =
                partial_of_lacks(vec![kinds_missing(&["variable"]), kinds_missing(&["enum"])])
            else {
                return Err(());
            };
            lacks
                .iter()
                .map(|each| match &each.missing {
                    MissingRows::Kinds { kinds, .. } => kinds.len(),
                    MissingRows::Any => 0,
                })
                .sum()
        }
        Scenario::NodeColumnsWithoutAFile => node_columns_without_a_file(),
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_coverage_partial_in_some_files_names_each_file_once"),
        word: Word::Succeeds,
        input: || Scenario::FilesNamed,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_file_nothing_is_said_of_lacks_any_row"),
        word: Word::Succeeds,
        input: || Scenario::UnnamedFileLacksAnyRow,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_kinds_missing_from_a_file_are_the_union_of_what_was_said_of_it"),
        word: Word::Succeeds,
        input: || Scenario::KindsUnited,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("every_relation_with_a_node_column_names_the_file_its_nodes_lie_in"),
        word: Word::Succeeds,
        input: || Scenario::NodeColumnsWithoutAFile,
        expect: Expect::Value(0),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
