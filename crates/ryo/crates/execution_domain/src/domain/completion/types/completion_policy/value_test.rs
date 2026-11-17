use super::value::*;
use crate::domain::completion::types::{
    CoverageState, MissingRows, ObservationAdmission, RegionAdmission,
};
use crate::domain::coverage::types::{FilePartial, Lacking};
use crate::domain::coverage::types::{LackingKind, Partial, Partials, RuleIndex};
use crate::domain::trust::types::relation_read::value::RelationRead;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{plan_parts::types::ColumnIndex, stored_facts::types::PartialCause};
use std::borrow::Cow;
fn partial() -> Partial {
    Partial {
        relation: Relation::SourceLocation,
        cause: PartialCause("fixture missing facts".into()),
        lacking: Lacking::Any,
        at: crate::domain::coverage::types::PartialPlace::File(FileId(0)),
    }
}
fn needs() -> Vec<RelationRead> {
    vec![RelationRead::whole(Relation::SourceLocation)]
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
    /// A project policy whose rule skipped files 2, 1 and 2 again: the files it withholds the rows of.
    SkippedFiles,
}

/// A file policy's admissions of `source_location` and the coverage it reports when blocked.
#[derive(Debug, PartialEq)]
struct Admissions {
    region: RegionAdmission,
    observation: ObservationAdmission,
    blocked: Coverage,
}

/// What a policy answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Coverage(Coverage),
    Admissions(Admissions),
    Files(Cow<'static, [FileId]>),
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
        Asked::SkippedFiles => {
            let partials = Partials {
                entries: vec![],
                unavailable_files: vec![FileId(2), FileId(1), FileId(2)],
            };
            Answer::Files(Cow::Owned(
                CompletionPolicy::project_inputs(RuleIndex(4), &partials, &partials)
                    .skipped_files(),
            ))
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
        name: CaseName("a_project_policy_names_each_file_its_rule_skipped_once_in_order"),
        word: Word::Succeeds,
        input: || Asked::SkippedFiles,
        expect: Expect::Value(Answer::Files(Cow::Borrowed(&[FileId(1), FileId(2)]))),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
