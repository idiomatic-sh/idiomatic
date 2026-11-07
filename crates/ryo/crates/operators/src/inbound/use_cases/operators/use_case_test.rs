//! Public inbound acceptance: actual semantic dispatch of row calls and borrowed outputs.
use super::{interface::*, use_case::OperatorsUseCaseImpl};
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::borrow::Cow;

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
        expect: Expect::Error(Failure::NoSuchColumn { index: 2 }),
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
        expect: Expect::Error(Failure::NoSuchColumn { index: 9 }),
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
        expect: Expect::Error(Failure::NoSuchColumn { index: 4 }),
    },
];
fn written(call: OperatorCall<'_>) -> Result<(), Failure> {
    assert_eq!(
        OperatorsUseCaseImpl::new().operators(call)?,
        OperatorAnswer::Written
    );
    Ok(())
}
fn subject(input: Input) -> Result<Value, Failure> {
    let mut target = Vec::with_capacity(8);
    let original = target.as_ptr();
    match input {
        Scenario::Project | Scenario::Append | Scenario::MissingProjectColumn => {
            let call = match input {
                Scenario::Project => OperatorCall::Project {
                    row: &ROW,
                    columns: &[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
                    target: &mut target,
                },
                Scenario::Append => OperatorCall::Append {
                    row: &ROW,
                    appended: &[ROW[0]],
                    target: &mut target,
                },
                Scenario::MissingProjectColumn => OperatorCall::Project {
                    row: &ROW,
                    columns: &[ColumnIndex(2)],
                    target: &mut target,
                },
                _ => unreachable!(),
            };
            written(call)?;
        }
        Scenario::SelectRepeated
        | Scenario::SelectRejected
        | Scenario::SelectOriginal
        | Scenario::SelectMissing => {
            target.push(ROW[0]);
            let kept: &[ColumnIndex] = match input {
                Scenario::SelectRepeated => &[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
                Scenario::SelectOriginal => &[],
                _ => &[ColumnIndex(9), ColumnIndex(8)],
            };
            let answer = OperatorsUseCaseImpl::new().operators(OperatorCall::Select {
                row: &ROW,
                kept,
                holds: !matches!(input, Scenario::SelectRejected),
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
            return Ok(Observation::Selected(accepted, Cow::Owned(target)));
        }
        Scenario::CountOutput | Scenario::DigestOutput | Scenario::AggregateMissing => {
            let (keys, count, digest): (&[ColumnIndex], _, _) = match input {
                Scenario::CountOutput => {
                    (&[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)], 4, None)
                }
                Scenario::DigestOutput => (&[], 3, Some(-5)),
                _ => (&[ColumnIndex(4), ColumnIndex(3)], 0, None),
            };
            written(OperatorCall::Aggregate {
                row: &ROW,
                keys,
                count,
                digest,
                target: &mut target,
            })?;
        }
        _ => unreachable!("row fixture receives only row scenarios"),
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
