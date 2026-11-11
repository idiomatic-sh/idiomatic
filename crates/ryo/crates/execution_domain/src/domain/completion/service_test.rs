//! The `completion` service's cases: what a coverage lacks, and the file a node of a partial lies in.

use std::borrow::Cow;

use super::service::{contributed, lacks_of, partial_any, partial_of_lacks};
use super::types::{CoverageState, FileLacks, MissingRows};
use crate::domain::coverage::types::LackingKind;
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The coverages of the inputs a publication was computed from.
#[derive(Clone, Copy, Debug)]
enum Inputs {
    /// Two complete inputs.
    Complete,
    /// A complete input and one partial in files 1 and 2, file 1 lacking only rows of the kind `variable`.
    PartialWithLacks,
    /// Two inputs inapplicable in files 3 and 4, and 4 and 5.
    Inapplicable,
    /// The partial input above and one inapplicable in file 3.
    PartialAndInapplicable,
}

/// What a row asks of completion.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// The coverage partial in these files, each lacking any row.
    PartialAny(&'static [Option<FileId>]),
    /// What these files lack when nothing was said of any of them.
    LacksOf(&'static [Option<FileId>]),
    /// The coverage partial in file 1, which two things said lack only rows of these kinds in column 1.
    PartialOfLacks(&'static [&'static str]),
    /// The relations of the catalog with a node column and not exactly one file column.
    NodeColumnsWithoutAFile,
    /// What inputs of these coverages contribute.
    Contributed(Inputs),
}

/// Which rows of a file are missing: any, or only those whose column holds one of these kinds.
#[derive(Clone, Debug, PartialEq)]
enum Missing {
    Any,
    Kinds(u32, Cow<'static, [Cow<'static, str>]>),
}

/// A file (none: the repository) and the rows it lacks.
type Lack = (Option<FileId>, Missing);

/// A coverage, as plain values.
#[derive(Clone, Debug, PartialEq)]
enum Coverage {
    Complete,
    Partial {
        files: Cow<'static, [Option<FileId>]>,
        lacks: Cow<'static, [Lack]>,
    },
    Inapplicable(Cow<'static, [FileId]>),
}

/// What completion answers.
#[derive(Clone, Debug, PartialEq)]
enum Answer {
    Coverage(Coverage),
    Lacks(Cow<'static, [Lack]>),
    Relations(Cow<'static, [Relation]>),
}

type Input = Ask;
type Value = Answer;
type Failure = OperatingError;

fn lack(each: &FileLacks) -> Lack {
    (
        each.file,
        match &each.missing {
            MissingRows::Any => Missing::Any,
            MissingRows::Kinds { column, kinds } => Missing::Kinds(
                column.0,
                kinds
                    .iter()
                    .map(|kind| Cow::Owned(kind.0.to_string()))
                    .collect(),
            ),
        },
    )
}

fn coverage(state: CoverageState) -> Coverage {
    match state {
        CoverageState::Complete => Coverage::Complete,
        CoverageState::Partial { files, lacks } => Coverage::Partial {
            files: files.into(),
            lacks: lacks.iter().map(lack).collect(),
        },
        CoverageState::Inapplicable { files } => Coverage::Inapplicable(files.into()),
    }
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

/// What the catalog lets a partial's file place: a node lies in the file its row names, so every relation with a node column has a
/// file column to lie in. (A node column that points into another file would escape the file a partial scopes to; the catalog does
/// not say which file a node column is of, so only this shape is asserted and the rest is the relation's own contract.)
fn node_columns_without_a_file() -> Cow<'static, [Relation]> {
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
        .copied()
        .collect()
}

fn subject(ask: Input) -> Result<Value, Failure> {
    Ok(match ask {
        Ask::PartialAny(files) => Answer::Coverage(coverage(partial_any(files.to_vec()))),
        Ask::LacksOf(files) => Answer::Lacks(lacks_of(files, &[]).iter().map(lack).collect()),
        Ask::PartialOfLacks(kinds) => Answer::Coverage(coverage(partial_of_lacks(
            kinds.iter().map(|kind| kinds_missing(&[kind])).collect(),
        ))),
        Ask::NodeColumnsWithoutAFile => Answer::Relations(node_columns_without_a_file()),
        Ask::Contributed(inputs) => Answer::Coverage(coverage(contributed(&coverages(inputs))?)),
    })
}

/// File 1 lacking only rows of the kind `variable` in column 1.
const VARIABLES_OF_FILE_1: Lack = (
    Some(FileId(1)),
    Missing::Kinds(1, Cow::Borrowed(&[Cow::Borrowed("variable")])),
);

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_coverage_partial_in_some_files_names_each_file_once"),
        word: Word::Succeeds,
        input: || Ask::PartialAny(&[Some(FileId(1)), None, Some(FileId(1))]),
        expect: Expect::Value(Answer::Coverage(Coverage::Partial {
            files: Cow::Borrowed(&[None, Some(FileId(1))]),
            lacks: Cow::Borrowed(&[]),
        })),
    },
    Case {
        name: CaseName("a_file_nothing_is_said_of_lacks_any_row"),
        word: Word::Succeeds,
        input: || Ask::LacksOf(&[Some(FileId(1))]),
        expect: Expect::Value(Answer::Lacks(Cow::Borrowed(&[(
            Some(FileId(1)),
            Missing::Any,
        )]))),
    },
    Case {
        name: CaseName("the_kinds_missing_from_a_file_are_the_union_of_what_was_said_of_it"),
        word: Word::Succeeds,
        input: || Ask::PartialOfLacks(&["variable", "enum"]),
        expect: Expect::Value(Answer::Coverage(Coverage::Partial {
            files: Cow::Borrowed(&[Some(FileId(1))]),
            lacks: Cow::Borrowed(&[(
                Some(FileId(1)),
                Missing::Kinds(
                    1,
                    Cow::Borrowed(&[Cow::Borrowed("enum"), Cow::Borrowed("variable")]),
                ),
            )]),
        })),
    },
    Case {
        name: CaseName("every_relation_with_a_node_column_names_the_file_its_nodes_lie_in"),
        word: Word::Succeeds,
        input: || Ask::NodeColumnsWithoutAFile,
        expect: Expect::Value(Answer::Relations(Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("complete_inputs_contribute_a_complete_coverage"),
        word: Word::Succeeds,
        input: || Ask::Contributed(Inputs::Complete),
        expect: Expect::Value(Answer::Coverage(Coverage::Complete)),
    },
    Case {
        name: CaseName("a_partial_input_contributes_its_files_and_their_lacks"),
        word: Word::Succeeds,
        input: || Ask::Contributed(Inputs::PartialWithLacks),
        expect: Expect::Value(Answer::Coverage(Coverage::Partial {
            files: Cow::Borrowed(&[Some(FileId(1)), Some(FileId(2))]),
            lacks: Cow::Borrowed(&[VARIABLES_OF_FILE_1]),
        })),
    },
    Case {
        name: CaseName("inapplicable_inputs_contribute_each_file_once"),
        word: Word::Succeeds,
        input: || Ask::Contributed(Inputs::Inapplicable),
        expect: Expect::Value(Answer::Coverage(Coverage::Inapplicable(Cow::Borrowed(&[
            FileId(3),
            FileId(4),
            FileId(5),
        ])))),
    },
    Case {
        name: CaseName("a_partial_input_makes_an_inapplicable_file_lack_any_row"),
        word: Word::Succeeds,
        input: || Ask::Contributed(Inputs::PartialAndInapplicable),
        expect: Expect::Value(Answer::Coverage(Coverage::Partial {
            files: Cow::Borrowed(&[Some(FileId(1)), Some(FileId(2)), Some(FileId(3))]),
            lacks: Cow::Borrowed(&[VARIABLES_OF_FILE_1]),
        })),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
