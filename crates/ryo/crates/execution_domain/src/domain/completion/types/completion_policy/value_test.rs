use super::value::*;
use crate::domain::completion::types::{
    CoverageState, MissingRows, ObservationAdmission, RegionAdmission,
};
use crate::domain::coverage::types::{FilePartial, Lacking};
use crate::domain::coverage::types::{LackingKind, Partial, Partials, RuleIndex};
use crate::domain::trust::types::RelationRead;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput},
    rows::types::RowCount,
    stored_facts::types::PartialCause,
};
use ryo_values::domain::{plans::types::Operation, rows::types::Table};
use std::borrow::Cow;
fn partial() -> Partial {
    Partial {
        relation: Relation::SourceLocation,
        cause: PartialCause("fixture missing facts".into()),
        lacking: Lacking::Any,
        at: crate::domain::coverage::types::PartialPlace::File(FileId(0)),
    }
}
fn anti() -> Operation {
    Operation::Antijoin {
        left: PlanInput::Relation(Relation::SourceLocation),
        right: PlanInput::Relation(Relation::SourceLocation),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
    }
}
fn needs() -> Vec<RelationRead> {
    vec![RelationRead {
        relation: Relation::SourceLocation,
        kinds: vec![],
    }]
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

/// What a policy is asked.
enum Asked {
    /// The coverage of `defines` read as it is, partly read in file 0 for what the partial lacks.
    RelationCoverage(Lacking),
    /// A file policy whose evaluation is partial and whose publication inputs are complete: its
    /// admissions and its blocked coverage.
    EvaluationRefusal,
    /// A project policy partly read in file 0 filtering an antijoin over the rows of files 0 and 1.
    ProjectFilter,
}

/// A file policy's admissions of `source_location` and the coverage it reports when blocked.
#[derive(Debug, PartialEq)]
struct Admissions {
    region: RegionAdmission,
    observation: ObservationAdmission,
    blocked: Coverage,
}

/// The rows a project policy's filter keeps, and the coverage it reports.
#[derive(Debug, PartialEq)]
struct Filtered {
    cells: Cow<'static, [Cell]>,
    count: RowCount,
    coverage: Coverage,
}

/// What a policy answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Coverage(Coverage),
    Admissions(Admissions),
    Filtered(Filtered),
}

type Input = Asked;
type Value = Answer;
type Failure = std::convert::Infallible;

fn subject(asked: Input) -> Result<Value, Failure> {
    Ok(match asked {
        Asked::RelationCoverage(lacking) => {
            let partials = Partials {
                entries: vec![FilePartial {
                    file: crate::domain::coverage::types::PartialPlace::File(FileId(0)),
                    partial: Partial {
                        relation: Relation::Defines,
                        lacking,
                        ..partial()
                    },
                }],
                unavailable_files: vec![],
            };
            Answer::Coverage(coverage(
                CompletionPolicy::project_inputs(RuleIndex(4), &partials, &partials)
                    .relation_coverage(Relation::Defines),
            ))
        }
        Asked::EvaluationRefusal => {
            let evaluation = [partial()];
            let policy = CompletionPolicy::file_inputs(RuleIndex(3), FileId(0), &evaluation, &[]);
            Answer::Admissions(Admissions {
                region: policy.region_admission(&needs()),
                observation: policy.observation_admission(&needs()),
                blocked: coverage(policy.blocked_coverage()),
            })
        }
        Asked::ProjectFilter => {
            let partials = Partials {
                entries: vec![FilePartial {
                    file: crate::domain::coverage::types::PartialPlace::File(FileId(0)),
                    partial: partial(),
                }],
                unavailable_files: vec![],
            };
            let policy = CompletionPolicy::project_inputs(RuleIndex(4), &partials, &partials);
            let mut table = Table {
                kinds: vec![CellKind::File],
                cells: vec![Cell::File(FileId(0)), Cell::File(FileId(1))],
                count: RowCount(2),
            };
            policy.filter_rows(&anti(), &needs(), &mut table);
            Answer::Filtered(Filtered {
                cells: Cow::Owned(table.cells),
                count: table.count,
                coverage: coverage(policy.coverage(&needs())),
            })
        }
    })
}

/// File 0 partly read for any row.
const ANY_IN_FILE_0: Coverage = Coverage::Partial {
    files: Cow::Borrowed(&[Some(FileId(0))]),
    lacks: Cow::Borrowed(&[]),
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("relation_coverage_names_the_kind_a_partial_lacks"),
        word: Word::Succeeds,
        input: || Asked::RelationCoverage(Lacking::Kind(LackingKind("variable".into()))),
        expect: Expect::Value(Answer::Coverage(Coverage::Partial {
            files: Cow::Borrowed(&[Some(FileId(0))]),
            lacks: Cow::Borrowed(&[(
                Some(FileId(0)),
                Some((ColumnIndex(1), Cow::Borrowed(&[Cow::Borrowed("variable")]))),
            )]),
        })),
    },
    Case {
        name: CaseName("a_partial_that_names_no_kind_lacks_any_row"),
        word: Word::Succeeds,
        input: || Asked::RelationCoverage(Lacking::Any),
        expect: Expect::Value(Answer::Coverage(ANY_IN_FILE_0)),
    },
    Case {
        name: CaseName("evaluation_refusal_preserves_partial_coverage"),
        word: Word::Succeeds,
        input: || Asked::EvaluationRefusal,
        expect: Expect::Value(Answer::Admissions(Admissions {
            region: RegionAdmission::Blocked,
            observation: ObservationAdmission::Ready,
            blocked: ANY_IN_FILE_0,
        })),
    },
    Case {
        name: CaseName("project_filter_preserves_physical_coverage"),
        word: Word::Succeeds,
        input: || Asked::ProjectFilter,
        expect: Expect::Value(Answer::Filtered(Filtered {
            cells: Cow::Borrowed(&[Cell::File(FileId(1))]),
            count: RowCount(1),
            coverage: ANY_IN_FILE_0,
        })),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
