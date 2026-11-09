//! Public inbound acceptance: actual semantic dispatch of row calls and borrowed outputs.
use super::{interface::OperatorsUseCase, use_case::OperatorsUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::capture::types::*;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::contract::types::*;
use crate::domain::equality_join::types::*;
use crate::domain::ordered_existence::types::*;
use crate::domain::ordered_match::types::*;
use crate::domain::reduction::types::*;
use crate::domain::select::types::RowSelection;
use crate::domain::set_rows::types::*;
use crate::domain::source_matches::types::*;
use crate::domain::words::types::*;
use crate::inbound::use_cases::operators::types::*;
use ryo_relations::domain::relations::types::{Count, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::borrow::Cow;

#[cfg(test)]
mod aggregation_checks;
#[cfg(test)]
mod groups;
#[cfg(test)]
mod ordered;
#[cfg(test)]
mod ordered_existence;
#[cfg(test)]
mod positions;
#[cfg(test)]
mod source;
#[cfg(test)]
mod source_optional;

#[cfg(test)]
mod streaming;
#[cfg(test)]
mod text;

#[derive(Clone, Copy)]
enum Scenario {
    Project,
    Append,
    MissingProjectColumn,
    SelectRepeated,
    SelectRejected,
    SelectOriginal,
    SelectMissing,
    CountOutput,
    DigestOutput,
    AggregateMissing,
}
type Input = Scenario;
#[derive(Debug, PartialEq)]
enum Observation {
    Cells(Cow<'static, [Cell]>),
    Selected(bool, Cow<'static, [Cell]>),
}
type Value = Observation;
type Failure = PortableOperatorFault;
const ROW: [Cell; 2] = [Cell::Number(Integer(7)), Cell::Number(Integer(9))];
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("project_preserves_repeated_plan_column"),
        word: Word::Succeeds,
        input: || Scenario::Project,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[ROW[1], ROW[0], ROW[1]]))),
    },
    Case {
        name: CaseName("append_preserves_row_and_suffix_order"),
        word: Word::Succeeds,
        input: || Scenario::Append,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[ROW[0], ROW[1], ROW[0]]))),
    },
    Case {
        name: CaseName("project_reports_absent_column"),
        word: Word::Refuses,
        input: || Scenario::MissingProjectColumn,
        expect: Expect::Error(Failure::NoSuchColumn {
            index: ryo_values::domain::plan_parts::types::ColumnIndex(2),
        }),
    },
    Case {
        name: CaseName("select_dispatch_preserves_repeated_kept_columns"),
        word: Word::Succeeds,
        input: || Scenario::SelectRepeated,
        expect: Expect::Value(Observation::Selected(
            true,
            Cow::Borrowed(&[ROW[1], ROW[0], ROW[1]]),
        )),
    },
    Case {
        name: CaseName("select_refusal_never_reads_invalid_kept_columns"),
        word: Word::Succeeds,
        input: || Scenario::SelectRejected,
        expect: Expect::Value(Observation::Selected(false, Cow::Borrowed(&[ROW[0]]))),
    },
    Case {
        name: CaseName("select_empty_kept_preserves_original_without_copying"),
        word: Word::Succeeds,
        input: || Scenario::SelectOriginal,
        expect: Expect::Value(Observation::Selected(true, Cow::Borrowed(&[ROW[0]]))),
    },
    Case {
        name: CaseName("select_reports_first_missing_kept_column"),
        word: Word::Refuses,
        input: || Scenario::SelectMissing,
        expect: Expect::Error(Failure::NoSuchColumn {
            index: ryo_values::domain::plan_parts::types::ColumnIndex(9),
        }),
    },
    Case {
        name: CaseName("aggregate_count_dispatch_preserves_key_order"),
        word: Word::Succeeds,
        input: || Scenario::CountOutput,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            ROW[1],
            ROW[0],
            ROW[1],
            Cell::Number(Integer(4)),
        ]))),
    },
    Case {
        name: CaseName("aggregate_digest_precedes_count"),
        word: Word::Succeeds,
        input: || Scenario::DigestOutput,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            Cell::Number(Integer(-5)),
            Cell::Number(Integer(3)),
        ]))),
    },
    Case {
        name: CaseName("aggregate_reports_first_missing_key"),
        word: Word::Refuses,
        input: || Scenario::AggregateMissing,
        expect: Expect::Error(Failure::NoSuchColumn {
            index: ryo_values::domain::plan_parts::types::ColumnIndex(4),
        }),
    },
];
fn written(call: OperatorCall<'_>) -> Result<(), Failure> {
    assert_eq!(
        OperatorsUseCaseImpl::new(std::sync::Arc::new(
            crate::outbound::matching::factory::use_case()
        ))
        .operators(call)?,
        OperatorAnswer::Written
    );
    Ok(())
}
/// The Select answer over `ROW` with its first cell already in the target: kept or not, and the target.
fn selected(kept: &[ColumnIndex], holds: PredicateOutcome) -> Result<Value, Failure> {
    let mut target = Vec::with_capacity(8);
    let original = target.as_ptr();
    target.push(ROW[0]);
    let answer = OperatorsUseCaseImpl::new(std::sync::Arc::new(
        crate::outbound::matching::factory::use_case(),
    ))
    .operators(OperatorCall::Select {
        row: &ROW,
        kept,
        holds,
        target: &mut target,
    })?;
    let OperatorAnswer::Selected(accepted) = answer else {
        panic!("Select must return its acceptance outcome")
    };
    assert_eq!(
        target.as_ptr(),
        original,
        "selection borrows caller-owned scratch"
    );
    Ok(Observation::Selected(
        accepted == RowSelection::Kept,
        Cow::Owned(target),
    ))
}
fn subject(input: Input) -> Result<Value, Failure> {
    let mut target = Vec::with_capacity(8);
    let original = target.as_ptr();
    match input {
        Scenario::Project => written(OperatorCall::Project {
            row: &ROW,
            columns: &[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
            target: &mut target,
        })?,
        Scenario::Append => written(OperatorCall::Append {
            row: &ROW,
            appended: &[ROW[0]],
            target: &mut target,
        })?,
        Scenario::MissingProjectColumn => written(OperatorCall::Project {
            row: &ROW,
            columns: &[ColumnIndex(2)],
            target: &mut target,
        })?,
        Scenario::SelectRepeated => {
            return selected(
                &[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
                PredicateOutcome::Holds,
            );
        }
        Scenario::SelectRejected => {
            return selected(&[ColumnIndex(9), ColumnIndex(8)], PredicateOutcome::Fails);
        }
        Scenario::SelectOriginal => return selected(&[], PredicateOutcome::Holds),
        Scenario::SelectMissing => {
            return selected(&[ColumnIndex(9), ColumnIndex(8)], PredicateOutcome::Holds);
        }
        Scenario::CountOutput => written(OperatorCall::Aggregate {
            row: &ROW,
            keys: &[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
            count: Count(4),
            digest: None,
            target: &mut target,
        })?,
        Scenario::DigestOutput => written(OperatorCall::Aggregate {
            row: &ROW,
            keys: &[],
            count: Count(3),
            digest: Some(Integer(-5)),
            target: &mut target,
        })?,
        Scenario::AggregateMissing => written(OperatorCall::Aggregate {
            row: &ROW,
            keys: &[ColumnIndex(4), ColumnIndex(3)],
            count: Count(0),
            digest: None,
            target: &mut target,
        })?,
    }
    assert_eq!(
        target.as_ptr(),
        original,
        "borrowed output keeps its allocation"
    );
    Ok(Observation::Cells(Cow::Owned(target)))
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
