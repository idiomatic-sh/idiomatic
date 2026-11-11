//! Physical completeness survives file-frontier conversion.
use super::fixtures::{
    Coverage, Frontier, KIND, Lack, Operator, Partly, Place, Restoration, coverage, frontier,
    restored, through,
};
use super::service::file_frontier_coverage;
use crate::domain::completion::service::partial_any;
use crate::domain::completion::types::CoverageState;
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::selects;
use std::borrow::Cow;

/// What a row of the cases converts.
enum Conversion {
    /// A coverage of file 0's frontier, converted to the frontier coverage a proof keeps.
    Coverage(CoverageState),
    /// The `variable` partial of file 3 kept as a frontier proof, restored and read on.
    Restored,
    /// A coverage read through an operator over `defines`.
    Through(Operator, Partly),
}

/// What a conversion answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Frontier(Frontier),
    Restoration(Restoration),
    Coverage(Coverage),
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

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
