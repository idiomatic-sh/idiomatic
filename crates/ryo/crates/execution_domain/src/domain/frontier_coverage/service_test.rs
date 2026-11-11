// ignore: idiomatic.arch/file-length - one case table for the file frontier: the conversion, the restored proof and the kind partial read through operators share one coverage form a row can state, and each row states the whole coverage it answers.
//! Physical completeness survives file-frontier conversion.
use super::service::{file_frontier_coverage, restored_lacks};
use crate::domain::completion::service::{partial_any, partial_of_lacks};
use crate::domain::completion::types::{CoverageState, FileLacks, MissingRows};
use crate::domain::coverage::types::LackingKind;
use crate::domain::operating::errors::OperatingError;
use crate::domain::output_coverage::service::output_coverage;
use crate::domain::output_coverage::types::{InputCoverage, OperandOrdinal};
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as PlanInput, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, RepeatedRows, Side, StepId, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::selects;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::records::types::{FrontierAffected, FrontierCoverage, FrontierMissing},
};
use std::borrow::Cow;

/// The cell kinds of the `defines` rows every operator here reads: a node and two symbols.
const KINDS: [CellKind; 3] = [CellKind::Node, CellKind::Symbol, CellKind::Symbol];

/// The column of `defines` that holds a row's kind, as a row writes it.
const KIND: ColumnIndex = ColumnIndex(1);

/// The column of `defines` that holds a row's kind, as the trust model places it.
fn kind() -> ColumnIndex {
    crate::domain::trust::service::kind_place(Relation::Defines).unwrap()
}

/// What a row of the cases converts.
enum Conversion {
    /// A coverage of file 0's frontier, converted to the frontier coverage a proof keeps.
    Coverage(CoverageState),
    /// The `variable` partial of file 3 kept as a frontier proof, restored and read on.
    Restored,
    /// A coverage read through an operator over `defines`.
    Through(Operator, Partly),
}

/// An operator over the three `defines` columns.
#[derive(Clone, Copy)]
enum Operator {
    /// A select of the rows whose kind column equals the text.
    SelectKind(&'static str),
    /// A select of the rows whose name column (the one after the kind) equals `enum`.
    SelectName,
    /// A projection of the node column alone.
    ProjectNode,
    /// A join, as its second operand, on the name column, keeping the node column.
    JoinOnName,
}

/// How a coverage reads `defines`.
#[derive(Clone, Copy)]
enum Partly {
    /// File 1 lacks `variable` rows only.
    OnlyVariable,
    /// File 1 lacks `variable` rows only; file 0 lacks any row.
    AlsoAnyInFile0,
}

/// What a file lacks, as a row writes it: none for any row, or the kind column and its kinds.
type Lack = Option<(ColumnIndex, Cow<'static, [Cow<'static, str>]>)>;

/// A coverage state as a row writes it.
#[derive(Clone, Debug, PartialEq)]
enum Coverage {
    Complete,
    Partial {
        files: Cow<'static, [Option<FileId>]>,
        lacks: Cow<'static, [(Option<FileId>, Lack)]>,
    },
    Inapplicable(Cow<'static, [FileId]>),
}

/// A frontier coverage as a row writes it: each affected place, current file or not, with its lack.
#[derive(Clone, Debug, PartialEq)]
enum Frontier {
    Complete,
    Partial(Cow<'static, [(Place, Lack)]>),
}

/// Where a frontier proof places its missing rows.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Place {
    CurrentFile,
    Repository,
}

/// A frontier proof kept and restored: what each step answered.
#[derive(Debug, PartialEq)]
struct Restoration {
    /// The proof file 3's `variable` partial is kept as.
    stored: Frontier,
    /// The coverage the proof restores in file 3.
    restored: Coverage,
    /// The restored coverage through a select of `enum` kinds.
    through_enum: Coverage,
    /// The restored coverage through a select of `variable` kinds.
    through_variable: Coverage,
    /// The coverage the proof restores in file 4.
    in_file_4: Coverage,
}

/// What a conversion answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Frontier(Frontier),
    Restoration(Restoration),
    Coverage(Coverage),
}

fn coverage(state: CoverageState) -> Coverage {
    let lack = |missing: MissingRows| match missing {
        MissingRows::Any => None,
        MissingRows::Kinds { column, kinds } => Some((
            column,
            kinds
                .into_iter()
                .map(|kind| Cow::Owned(kind.0.into()))
                .collect(),
        )),
    };
    match state {
        CoverageState::Complete => Coverage::Complete,
        CoverageState::Partial { files, lacks } => Coverage::Partial {
            files: Cow::Owned(files),
            lacks: lacks
                .into_iter()
                .map(|lacks| (lacks.file, lack(lacks.missing)))
                .collect(),
        },
        CoverageState::Inapplicable { files } => Coverage::Inapplicable(Cow::Owned(files)),
    }
}

fn frontier(state: FrontierCoverage) -> Frontier {
    let lack = |missing: FrontierMissing| match missing {
        FrontierMissing::Any => None,
        FrontierMissing::Kinds { column, kinds } => Some((
            column,
            kinds
                .into_iter()
                .map(|kind| Cow::Owned(kind.0.into()))
                .collect(),
        )),
    };
    match state {
        FrontierCoverage::Complete => Frontier::Complete,
        FrontierCoverage::Partial(affected) => Frontier::Partial(
            affected
                .into_iter()
                .map(|affected| match affected {
                    FrontierAffected::CurrentFile(missing) => (Place::CurrentFile, lack(missing)),
                    FrontierAffected::Repository(missing) => (Place::Repository, lack(missing)),
                })
                .collect(),
        ),
    }
}

/// What a row of the cases asks.
type Input = Conversion;
type Value = Answer;
type Failure = OperatingError;

/// File `file` lacks `variable` rows only.
const fn variable_in(file: u32) -> (Option<FileId>, Lack) {
    (
        Some(FileId(file)),
        Some((KIND, Cow::Borrowed(&[Cow::Borrowed("variable")]))),
    )
}

/// A partial read of `defines` where only file 1 lacks `variable` rows.
const ONLY_VARIABLE: Coverage = Coverage::Partial {
    files: Cow::Borrowed(&[Some(FileId(1))]),
    lacks: Cow::Borrowed(const { &[variable_in(1)] }),
};

/// A partial read where each listed file lacks any row.
const fn any_in(files: &'static [Option<FileId>]) -> Coverage {
    Coverage::Partial {
        files: Cow::Borrowed(files),
        lacks: Cow::Borrowed(&[]),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_kind_partial_is_cleared_by_a_select_that_stops_the_kind_and_by_nothing_else",
        ),
        word: Word::Succeeds,
        input: || Conversion::Through(Operator::SelectKind("enum"), Partly::OnlyVariable),
        expect: Expect::Value(Answer::Coverage(Coverage::Complete)),
    },
    Case {
        name: CaseName("a_select_that_keeps_the_missing_kind_stays_partial_for_that_file"),
        word: Word::Succeeds,
        input: || Conversion::Through(Operator::SelectKind("variable"), Partly::OnlyVariable),
        expect: Expect::Value(Answer::Coverage(ONLY_VARIABLE)),
    },
    Case {
        name: CaseName("a_select_on_a_column_that_is_not_the_kind_column_rules_nothing_out"),
        word: Word::Succeeds,
        input: || Conversion::Through(Operator::SelectName, Partly::OnlyVariable),
        expect: Expect::Value(Answer::Coverage(ONLY_VARIABLE)),
    },
    Case {
        name: CaseName("the_file_that_lacks_any_row_stays_partial_and_is_the_only_one"),
        word: Word::Succeeds,
        input: || Conversion::Through(Operator::SelectKind("enum"), Partly::AlsoAnyInFile0),
        expect: Expect::Value(Answer::Coverage(any_in(&[Some(FileId(0))]))),
    },
    Case {
        name: CaseName("a_projection_that_drops_the_kind_column_no_longer_tells_which_rows"),
        word: Word::Succeeds,
        input: || Conversion::Through(Operator::ProjectNode, Partly::OnlyVariable),
        expect: Expect::Value(Answer::Coverage(any_in(&[Some(FileId(1))]))),
    },
    Case {
        name: CaseName("a_join_keyed_by_anything_else_still_widens_to_the_repository"),
        word: Word::Succeeds,
        input: || Conversion::Through(Operator::JoinOnName, Partly::OnlyVariable),
        expect: Expect::Value(Answer::Coverage(any_in(&[None]))),
    },
    Case {
        name: CaseName(
            "a_restored_frontier_proof_clears_for_an_untouched_kind_and_stays_for_a_touched_one_in_its_own_file",
        ),
        word: Word::Succeeds,
        input: || Conversion::Restored,
        expect: Expect::Value(Answer::Restoration(Restoration {
            stored: Frontier::Partial(Cow::Borrowed(&[(
                Place::CurrentFile,
                Some((KIND, Cow::Borrowed(&[Cow::Borrowed("variable")]))),
            )])),
            restored: Coverage::Partial {
                files: Cow::Borrowed(&[Some(FileId(3))]),
                lacks: Cow::Borrowed(const { &[variable_in(3)] }),
            },
            through_enum: Coverage::Complete,
            through_variable: Coverage::Partial {
                files: Cow::Borrowed(&[Some(FileId(3))]),
                lacks: Cow::Borrowed(const { &[variable_in(3)] }),
            },
            in_file_4: Coverage::Partial {
                files: Cow::Borrowed(&[Some(FileId(4))]),
                lacks: Cow::Borrowed(const { &[variable_in(4)] }),
            },
        })),
    },
    Case {
        name: CaseName("complete"),
        word: Word::Succeeds,
        input: || Conversion::Coverage(CoverageState::Complete),
        expect: Expect::Value(Answer::Frontier(Frontier::Complete)),
    },
    Case {
        name: CaseName("current_file"),
        word: Word::Succeeds,
        input: || {
            Conversion::Coverage(CoverageState::Partial {
                files: vec![Some(FileId(0))],
                lacks: vec![],
            })
        },
        expect: Expect::Value(Answer::Frontier(Frontier::Partial(Cow::Borrowed(&[(
            Place::CurrentFile,
            None,
        )])))),
    },
    Case {
        name: CaseName("repository"),
        word: Word::Succeeds,
        input: || Conversion::Coverage(partial_any(vec![None])),
        expect: Expect::Value(Answer::Frontier(Frontier::Partial(Cow::Borrowed(&[(
            Place::Repository,
            None,
        )])))),
    },
    Case {
        name: CaseName("duplicate_causes"),
        word: Word::Succeeds,
        input: || {
            Conversion::Coverage(CoverageState::Partial {
                files: vec![Some(FileId(0)), Some(FileId(0)), None],
                lacks: vec![],
            })
        },
        expect: Expect::Value(Answer::Frontier(Frontier::Partial(Cow::Borrowed(&[
            (Place::CurrentFile, None),
            (Place::Repository, None),
        ])))),
    },
    Case {
        name: CaseName("foreign_file"),
        word: Word::Refuses,
        input: || {
            Conversion::Coverage(CoverageState::Partial {
                files: vec![Some(FileId(1))],
                lacks: vec![],
            })
        },
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, OperatingError::ObservationIncomplete { reason } if reason.as_str() == "file frontier carries a foreign physical partition"),
            )
        }),
    },
    Case {
        name: CaseName("empty_partial"),
        word: Word::Refuses,
        input: || Conversion::Coverage(partial_any(vec![])),
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, OperatingError::ObservationIncomplete { reason } if reason.as_str() == "partial frontier has no affected physical partition"),
            )
        }),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Conversion::Coverage(state) => Ok(Answer::Frontier(frontier(file_frontier_coverage(
            state,
            FileId(0),
        )?))),
        Conversion::Through(operator, partly) => {
            through(operator, partly).map(|state| Answer::Coverage(coverage(state)))
        }
        Conversion::Restored => restored().map(Answer::Restoration),
    }
}

/// The `variable` lack of one file.
fn variable(file: u32) -> FileLacks {
    FileLacks {
        file: Some(FileId(file)),
        missing: MissingRows::Kinds {
            column: kind(),
            kinds: vec![LackingKind("variable".into())],
        },
    }
}

/// A select of the rows whose `column` equals `text`.
fn select(text: &str, column: ColumnIndex) -> Operation {
    Operation::Select {
        from: PlanInput::Step(StepId(0)),
        comparisons: vec![PlanComparison {
            test: Test::Equal,
            left: Operand::Column(column),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: Vec::new(),
    }
}

/// Reads `defines` as `partly` says through `operator`.
fn through(operator: Operator, partly: Partly) -> Result<CoverageState, OperatingError> {
    let read = match partly {
        Partly::OnlyVariable => partial_of_lacks(vec![variable(1)]),
        Partly::AlsoAnyInFile0 => CoverageState::Partial {
            files: vec![Some(FileId(0)), Some(FileId(1))],
            lacks: vec![variable(1)],
        },
    };
    let (ordinal, operation, out) = match operator {
        Operator::SelectKind(text) => (0, select(text, kind()), &KINDS[..]),
        Operator::SelectName => (0, select("enum", ColumnIndex(kind().0 + 1)), &KINDS[..]),
        Operator::ProjectNode => (
            0,
            Operation::Project {
                from: PlanInput::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Dropped,
            },
            &KINDS[..1],
        ),
        Operator::JoinOnName => (
            1,
            Operation::Join {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(1)),
                left_on: vec![ColumnIndex(1)],
                right_on: vec![ColumnIndex(1)],
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                }],
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
            },
            &KINDS[..1],
        ),
    };
    output_coverage(
        &operation,
        out,
        &[InputCoverage {
            ordinal: OperandOrdinal(ordinal),
            kinds: &KINDS,
            coverage: read,
            lineage: Default::default(),
        }],
    )
}

/// Keeps file 3's `variable` partial as a frontier proof, restores it in files 3 and 4, and reads the
/// restored coverage through two selects.
fn restored() -> Result<Restoration, OperatingError> {
    let kept = file_frontier_coverage(partial_of_lacks(vec![variable(3)]), FileId(3))?;
    let affected = match &kept {
        FrontierCoverage::Partial(affected) => affected.clone(),
        FrontierCoverage::Complete => Vec::new(),
    };
    let restored = partial_of_lacks(restored_lacks(FileId(3), &affected));
    let read = |text: &str| {
        output_coverage(
            &select(text, kind()),
            &KINDS,
            &[InputCoverage {
                ordinal: OperandOrdinal(0),
                kinds: &KINDS,
                coverage: restored.clone(),
                lineage: Default::default(),
            }],
        )
        .map(coverage)
    };
    Ok(Restoration {
        stored: frontier(kept),
        through_enum: read("enum")?,
        through_variable: read("variable")?,
        in_file_4: coverage(partial_of_lacks(restored_lacks(FileId(4), &affected))),
        restored: coverage(restored),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
