//! The `coverage` service's cases, on values alone.

use std::borrow::Cow;

use super::service::{affected_ordinals, drop_inapplicable, drop_uncertain, partial_of};
use super::types::{DeclaredFiles, Lacking, PartialPlace, Recovery};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Relation, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput, StepId};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;

/// The operation whose answer is the two rows of `rows`.
#[derive(Clone, Copy, Debug)]
enum Answering {
    /// An antijoin on the left column at the first place and the right column at the second.
    Antijoin(u32, u32),
    /// A join of the number column on the left with the first column on the right.
    Join,
}

/// What a row asks of coverage.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// The rows of the answer kept when these files could be missing rows.
    Uncertain(Answering, &'static [Option<FileId>]),
    /// The rows of the answer kept when these files are skipped.
    Skipped(Answering, &'static [FileId]),
    /// The relations a file is read in part in: the relation its reader refused, if any, how much of the file is a recovery,
    /// and the relations wanted of it.
    ReadInPart {
        refused: Option<Relation>,
        recovery: Recovery,
        wanted: &'static [Relation],
    },
    /// What a file so recovered leaves of the rows of an absence check keyed by name on byte literals.
    AbsenceOnByteLiterals(Recovery),
    /// The ordinals of the files a partial source names, over this many declared files.
    Affected(&'static [Option<FileId>], usize),
}

/// A relation read in part, which of its rows could be missing, and where.
type ReadPart = (Relation, Lacking, PartialPlace);

/// What coverage answers.
#[derive(Clone, Debug, PartialEq)]
enum Answer {
    /// The files of the rows kept, in order.
    Kept(Cow<'static, [u32]>),
    /// The relations read in part.
    ReadInPart(Cow<'static, [ReadPart]>),
    /// The relations read in part, and the files of the rows of the absence check kept.
    Absence {
        read_in_part: Cow<'static, [ReadPart]>,
        kept: Cow<'static, [u32]>,
    },
    /// The ordinals of the affected files.
    Ordinals(Cow<'static, [usize]>),
}

type Input = Ask;
type Value = Answer;
/// What the placing of a partial source is refused with.
type Failure = OperatingError;

fn operation(answering: Answering) -> Operation {
    match answering {
        Answering::Antijoin(left, right) => Operation::Antijoin {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Step(StepId(1)),
            left_on: vec![ColumnIndex(left)],
            right_on: vec![ColumnIndex(right)],
        },
        Answering::Join => Operation::Join {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Step(StepId(1)),
            left_on: vec![ColumnIndex(1)],
            right_on: vec![ColumnIndex(0)],
            output: Vec::new(),
            filter: Vec::new(),
            view: Vec::new(),
            reach: None,
        },
    }
}

/// Two rows, one of file 1 and one of file 2, under a column of files and a column of numbers.
fn rows() -> Table {
    let mut table = Table {
        kinds: vec![CellKind::File, CellKind::Number],
        ..Table::default()
    };
    push_row(
        &mut table,
        &[Cell::File(FileId(1)), Cell::Number(Integer(7))],
    );
    push_row(
        &mut table,
        &[Cell::File(FileId(2)), Cell::Number(Integer(7))],
    );
    table
}

/// The files of the rows of a table whose first column is a file.
fn files_of(table: &Table) -> Cow<'static, [u32]> {
    table
        .cells
        .chunks(table.kinds.len())
        .filter_map(|row| match row.first() {
            Some(Cell::File(file)) => Some(file.0),
            _ => None,
        })
        .collect()
}

fn read_in_part(
    refused: Option<Relation>,
    recovery: Recovery,
    wanted: &[Relation],
) -> Vec<ReadPart> {
    use ryo_relations::domain::relations::types::Missing;
    use ryo_values::domain::stored_facts::types::{PartialCause, PartialScope, ReadInPart};
    let said: Vec<ReadInPart> = refused
        .into_iter()
        .map(|relation| ReadInPart {
            relation,
            cause: PartialCause("reader stopped before EOF".into()),
            lacking: Missing::Any,
            at: PartialScope::Repository,
        })
        .collect();
    partial_of(&said, recovery, wanted)
        .into_iter()
        .map(|part| (part.relation, part.lacking, part.at))
        .collect()
}

fn subject(ask: Input) -> Result<Value, Failure> {
    let kinds = [CellKind::File, CellKind::Number];
    Ok(match ask {
        Ask::Uncertain(answering, files) => {
            let mut table = rows();
            drop_uncertain(&operation(answering), &kinds, &mut table, files);
            Answer::Kept(files_of(&table))
        }
        Ask::Skipped(answering, files) => {
            let mut table = rows();
            drop_inapplicable(&operation(answering), &kinds, &mut table, files);
            Answer::Kept(files_of(&table))
        }
        Ask::ReadInPart {
            refused,
            recovery,
            wanted,
        } => Answer::ReadInPart(read_in_part(refused, recovery, wanted).into()),
        Ask::AbsenceOnByteLiterals(recovery) => {
            let partial = read_in_part(None, recovery, &[Relation::BytesLiteral]);
            let mut absent = rows();
            let places: Vec<Option<FileId>> = partial
                .iter()
                .map(|(_, _, at)| match at {
                    PartialPlace::File(file) => Some(*file),
                    PartialPlace::Repository => None,
                })
                .collect();
            drop_uncertain(
                &operation(Answering::Antijoin(1, 1)),
                &kinds,
                &mut absent,
                &places,
            );
            Answer::Absence {
                read_in_part: partial.into(),
                kept: files_of(&absent),
            }
        }
        Ask::Affected(files, declared) => Answer::Ordinals(
            affected_ordinals(files, DeclaredFiles(declared))?
                .iter()
                .map(|ordinal| ordinal.0)
                .collect(),
        ),
    })
}

/// The relations `partial_of` is asked of a file in the recovery rows.
const CODE_AND_TEXT: &[Relation] = &[
    Relation::Comment,
    Relation::IndentRegion,
    Relation::Function,
];

/// Each of the code-and-text relations, in the order the service names them, read in part in the whole repository with any row lacking.
const EVERY_CODE_AND_TEXT_RELATION: &[ReadPart] = &[
    (Relation::Function, Lacking::Any, PartialPlace::Repository),
    (Relation::Comment, Lacking::Any, PartialPlace::Repository),
    (
        Relation::IndentRegion,
        Lacking::Any,
        PartialPlace::Repository,
    ),
];

/// Byte literals, read in part in the whole repository with any row lacking.
const BYTE_LITERALS_IN_PART: &[ReadPart] = &[(
    Relation::BytesLiteral,
    Lacking::Any,
    PartialPlace::Repository,
)];

/// Byte literals read in part, and no row of the absence check kept.
const ABSENCE_REFUSED: Answer = Answer::Absence {
    read_in_part: Cow::Borrowed(BYTE_LITERALS_IN_PART),
    kept: Cow::Borrowed(&[]),
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_row_keyed_by_a_file_is_dropped_only_if_its_file_could_be_missing_rows"),
        word: Word::Succeeds,
        input: || Ask::Uncertain(Answering::Antijoin(0, 0), &[Some(FileId(2))]),
        expect: Expect::Value(Answer::Kept(Cow::Borrowed(&[1]))),
    },
    Case {
        name: CaseName("a_row_with_no_file_key_is_dropped_whichever_file_could_be_missing_rows"),
        word: Word::Succeeds,
        input: || Ask::Uncertain(Answering::Antijoin(1, 1), &[Some(FileId(2))]),
        expect: Expect::Value(Answer::Kept(Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("a_skipped_file_drops_no_row_of_a_join"),
        word: Word::Succeeds,
        input: || Ask::Skipped(Answering::Join, &[FileId(2)]),
        expect: Expect::Value(Answer::Kept(Cow::Borrowed(&[1, 2]))),
    },
    Case {
        name: CaseName("a_skipped_file_drops_only_its_own_rows_of_an_antijoin_keyed_by_file"),
        word: Word::Succeeds,
        input: || Ask::Skipped(Answering::Antijoin(0, 0), &[FileId(2)]),
        expect: Expect::Value(Answer::Kept(Cow::Borrowed(&[1]))),
    },
    Case {
        name: CaseName("an_antijoin_reads_its_file_key_from_its_answer"),
        word: Word::Succeeds,
        input: || Ask::Skipped(Answering::Antijoin(0, 1), &[FileId(2)]),
        expect: Expect::Value(Answer::Kept(Cow::Borrowed(&[1]))),
    },
    Case {
        name: CaseName("a_file_with_syntax_errors_is_read_in_part_in_the_relations_of_code_only"),
        word: Word::Succeeds,
        input: || Ask::ReadInPart {
            refused: None,
            recovery: Recovery::SyntaxErrors,
            wanted: &[Relation::Defines, Relation::FilesystemFile],
        },
        expect: Expect::Value(Answer::ReadInPart(Cow::Borrowed(&[(
            Relation::Defines,
            Lacking::Any,
            PartialPlace::Repository,
        )]))),
    },
    Case {
        name: CaseName("lexical_recovery_keeps_explicit_reader_refusals_and_code_uncertainty"),
        word: Word::Succeeds,
        input: || Ask::ReadInPart {
            refused: Some(Relation::Comment),
            recovery: Recovery::LexicallyCompleteSyntaxErrors,
            wanted: CODE_AND_TEXT,
        },
        expect: Expect::Value(Answer::ReadInPart(Cow::Borrowed(&[
            (Relation::Function, Lacking::Any, PartialPlace::Repository),
            (Relation::Comment, Lacking::Any, PartialPlace::Repository),
        ]))),
    },
    Case {
        name: CaseName("lexical_recovery_without_a_reader_refusal_keeps_only_code_uncertainty"),
        word: Word::Succeeds,
        input: || Ask::ReadInPart {
            refused: None,
            recovery: Recovery::LexicallyCompleteSyntaxErrors,
            wanted: CODE_AND_TEXT,
        },
        expect: Expect::Value(Answer::ReadInPart(Cow::Borrowed(&[(
            Relation::Function,
            Lacking::Any,
            PartialPlace::Repository,
        )]))),
    },
    Case {
        name: CaseName("a_file_that_is_only_a_recovery_is_partial_in_every_wanted_relation"),
        word: Word::Succeeds,
        input: || Ask::ReadInPart {
            refused: None,
            recovery: Recovery::SyntaxErrors,
            wanted: CODE_AND_TEXT,
        },
        expect: Expect::Value(Answer::ReadInPart(Cow::Borrowed(
            EVERY_CODE_AND_TEXT_RELATION,
        ))),
    },
    Case {
        name: CaseName("an_unparsed_file_is_partial_in_every_wanted_relation"),
        word: Word::Succeeds,
        input: || Ask::ReadInPart {
            refused: None,
            recovery: Recovery::Unparsed,
            wanted: CODE_AND_TEXT,
        },
        expect: Expect::Value(Answer::ReadInPart(Cow::Borrowed(
            EVERY_CODE_AND_TEXT_RELATION,
        ))),
    },
    Case {
        name: CaseName("recovered_byte_literals_refuse_absence"),
        word: Word::Succeeds,
        input: || Ask::AbsenceOnByteLiterals(Recovery::SyntaxErrors),
        expect: Expect::Value(ABSENCE_REFUSED),
    },
    Case {
        name: CaseName("lexically_recovered_byte_literals_refuse_absence"),
        word: Word::Succeeds,
        input: || Ask::AbsenceOnByteLiterals(Recovery::LexicallyCompleteSyntaxErrors),
        expect: Expect::Value(ABSENCE_REFUSED),
    },
    Case {
        name: CaseName("unparsed_byte_literals_refuse_absence"),
        word: Word::Succeeds,
        input: || Ask::AbsenceOnByteLiterals(Recovery::Unparsed),
        expect: Expect::Value(ABSENCE_REFUSED),
    },
    Case {
        name: CaseName("whole_source_keeps_the_same_result_of_an_absence_check"),
        word: Word::Succeeds,
        input: || Ask::AbsenceOnByteLiterals(Recovery::Whole),
        expect: Expect::Value(Answer::Absence {
            read_in_part: Cow::Borrowed(&[]),
            kept: Cow::Borrowed(&[1, 2]),
        }),
    },
    Case {
        name: CaseName("partial_project_coverage_names_each_affected_file_once"),
        word: Word::Succeeds,
        input: || Ask::Affected(&[Some(FileId(2)), Some(FileId(0)), Some(FileId(2))], 3),
        expect: Expect::Value(Answer::Ordinals(Cow::Borrowed(&[0, 2]))),
    },
    Case {
        name: CaseName("an_unlocated_partial_source_names_every_declared_file"),
        word: Word::Succeeds,
        input: || Ask::Affected(&[None, Some(FileId(2))], 3),
        expect: Expect::Value(Answer::Ordinals(Cow::Borrowed(&[0, 1, 2]))),
    },
    Case {
        name: CaseName("a_partial_source_naming_no_file_is_refused"),
        word: Word::Refuses,
        input: || Ask::Affected(&[], 3),
        expect: Expect::Refusal(is_unplaceable),
    },
    Case {
        name: CaseName("a_partial_source_over_no_declared_file_is_refused"),
        word: Word::Refuses,
        input: || Ask::Affected(&[None], 0),
        expect: Expect::Refusal(is_unplaceable),
    },
    Case {
        name: CaseName("a_file_beyond_the_declared_files_is_refused"),
        word: Word::Refuses,
        input: || Ask::Affected(&[Some(FileId(3))], 3),
        expect: Expect::Refusal(is_unplaceable),
    },
    Case {
        name: CaseName("an_unlocated_source_beside_a_file_beyond_the_declared_files_is_refused"),
        word: Word::Refuses,
        input: || Ask::Affected(&[None, Some(FileId(999))], 3),
        expect: Expect::Refusal(is_unplaceable),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// Whether an error is the refusal to place a partial source on the declared files.
fn is_unplaceable(error: &OperatingError) -> ryo_values::cases::Selection {
    ryo_values::selects!(matches!(
        error,
        OperatingError::ObservationIncomplete { .. }
    ))
}
