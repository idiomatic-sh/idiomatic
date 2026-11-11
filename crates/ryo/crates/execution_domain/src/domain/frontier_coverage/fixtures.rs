//! The coverage and frontier forms a row of the frontier cases states, and the operators and proofs it reads them through.
use super::service::{file_frontier_coverage, restored_lacks};
use crate::domain::completion::service::partial_of_lacks;
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
use ryo_values::domain::records::types::{FrontierAffected, FrontierCoverage, FrontierMissing};
use std::borrow::Cow;

/// The cell kinds of the `defines` rows every operator here reads: a node and two symbols.
pub(super) const KINDS: [CellKind; 3] = [CellKind::Node, CellKind::Symbol, CellKind::Symbol];

/// The column of `defines` that holds a row's kind, as a row writes it.
pub(super) const KIND: ColumnIndex = ColumnIndex(1);

/// The column of `defines` that holds a row's kind, as the trust model places it.
pub(super) fn kind() -> ColumnIndex {
    crate::domain::trust::service::kind_place(Relation::Defines).unwrap()
}

/// An operator over the three `defines` columns.
#[derive(Clone, Copy)]
pub(super) enum Operator {
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
pub(super) enum Partly {
    /// File 1 lacks `variable` rows only.
    OnlyVariable,
    /// File 1 lacks `variable` rows only; file 0 lacks any row.
    AlsoAnyInFile0,
}

/// What a file lacks, as a row writes it: none for any row, or the kind column and its kinds.
pub(super) type Lack = Option<(ColumnIndex, Cow<'static, [Cow<'static, str>]>)>;

/// A coverage state as a row writes it.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Coverage {
    Complete,
    Partial {
        files: Cow<'static, [Option<FileId>]>,
        lacks: Cow<'static, [(Option<FileId>, Lack)]>,
    },
    Inapplicable(Cow<'static, [FileId]>),
}

/// A frontier coverage as a row writes it: each affected place, current file or not, with its lack.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Frontier {
    Complete,
    Partial(Cow<'static, [(Place, Lack)]>),
}

/// Where a frontier proof places its missing rows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Place {
    CurrentFile,
    Repository,
}

/// A frontier proof kept and restored: what each step answered.
#[derive(Debug, PartialEq)]
pub(super) struct Restoration {
    /// The proof file 3's `variable` partial is kept as.
    pub(super) stored: Frontier,
    /// The coverage the proof restores in file 3.
    pub(super) restored: Coverage,
    /// The restored coverage through a select of `enum` kinds.
    pub(super) through_enum: Coverage,
    /// The restored coverage through a select of `variable` kinds.
    pub(super) through_variable: Coverage,
    /// The coverage the proof restores in file 4.
    pub(super) in_file_4: Coverage,
}

pub(super) fn coverage(state: CoverageState) -> Coverage {
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

pub(super) fn frontier(state: FrontierCoverage) -> Frontier {
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
pub(super) fn through(operator: Operator, partly: Partly) -> Result<CoverageState, OperatingError> {
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
pub(super) fn restored() -> Result<Restoration, OperatingError> {
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
