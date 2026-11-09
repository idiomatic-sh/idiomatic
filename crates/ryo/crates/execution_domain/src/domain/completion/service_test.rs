//! The `completion` service's cases: what a coverage lacks, and the file a node of a partial lies in.

use super::service::{contributed, lacks_of, partial_any, partial_of_lacks};
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
    /// What inputs of these coverages contribute: complete is 0, inapplicable 100 plus its files, partial 200 plus ten times its
    /// files plus its lacks.
    Contributed(Inputs),
}

/// The coverages of the inputs a publication was computed from.
#[derive(Clone, Copy)]
enum Inputs {
    /// Two complete inputs.
    Complete,
    /// A complete input and one partial in two files, one of which lacks only rows of some kinds.
    PartialWithLacks,
    /// Two inputs inapplicable in overlapping files.
    Inapplicable,
    /// A partial input and an inapplicable one.
    PartialAndInapplicable,
}

fn coverages(inputs: Inputs) -> Vec<CoverageState> {
    let partial = || CoverageState::Partial {
        files: vec![Some(FileId(1)), Some(FileId(2))],
        lacks: vec![kinds_missing(&["variable"])],
    };
    let inapplicable = |files: &[u32]| CoverageState::Inapplicable {
        files: files.iter().map(|file| FileId(*file)).collect(),
    };
    match inputs {
        Inputs::Complete => vec![CoverageState::Complete, CoverageState::Complete],
        Inputs::PartialWithLacks => vec![CoverageState::Complete, partial()],
        Inputs::Inapplicable => vec![inapplicable(&[3, 4]), inapplicable(&[4, 5])],
        Inputs::PartialAndInapplicable => vec![partial(), inapplicable(&[3])],
    }
}

fn contributed_code(inputs: Inputs) -> Result<Value, Failure> {
    Ok(match contributed(&coverages(inputs)).map_err(|_| ())? {
        CoverageState::Complete => 0,
        CoverageState::Inapplicable { files } => 100 + files.len(),
        CoverageState::Partial { files, lacks } => 200 + files.len() * 10 + lacks.len(),
    })
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
        Scenario::Contributed(inputs) => contributed_code(inputs)?,
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
    Case {
        name: CaseName("complete_inputs_contribute_a_complete_coverage"),
        word: Word::Succeeds,
        input: || Scenario::Contributed(Inputs::Complete),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_partial_input_contributes_its_files_and_their_lacks"),
        word: Word::Succeeds,
        input: || Scenario::Contributed(Inputs::PartialWithLacks),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("inapplicable_inputs_contribute_each_file_once"),
        word: Word::Succeeds,
        input: || Scenario::Contributed(Inputs::Inapplicable),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_partial_input_makes_an_inapplicable_file_lack_any_row"),
        word: Word::Succeeds,
        input: || Scenario::Contributed(Inputs::PartialAndInapplicable),
        expect: Expect::Value(0),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
