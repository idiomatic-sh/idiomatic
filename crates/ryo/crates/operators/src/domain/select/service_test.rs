//! PredicateOutcome refusal does not inspect invalid kept columns.
use super::service::select;
use super::types::{PredicateOutcome, RowSelection};
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
type Input = (bool, &'static [ColumnIndex]);
type Value = (bool, std::borrow::Cow<'static, [Cell]>);
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("failed_predicate_does_not_read_missing_column"),
        word: Word::Succeeds,
        input: || (false, &[ColumnIndex(9)]),
        expect: Expect::Value((false, std::borrow::Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("empty_kept_preserves_original_row_without_copy"),
        word: Word::Succeeds,
        input: || (true, &[]),
        expect: Expect::Value((true, std::borrow::Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("repeated_columns_keep_authored_order"),
        word: Word::Succeeds,
        input: || (true, &[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)]),
        expect: Expect::Value((
            true,
            std::borrow::Cow::Borrowed(&[
                Cell::Number(Integer(9)),
                Cell::Number(Integer(7)),
                Cell::Number(Integer(9)),
            ]),
        )),
    },
    Case {
        name: CaseName("held_predicate_reports_first_absent_column"),
        word: Word::Refuses,
        input: || (true, &[ColumnIndex(9), ColumnIndex(8)]),
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: ryo_values::domain::plan_parts::types::ColumnIndex(9) }),
    },
];
fn subject((holds, kept): Input) -> Result<Value, Failure> {
    let mut target = Vec::new();
    let accepted = select(
        &[Cell::Number(Integer(7)), Cell::Number(Integer(9))],
        kept,
        if holds {
            PredicateOutcome::Holds
        } else {
            PredicateOutcome::Fails
        },
        &mut target,
    )?;
    Ok((
        accepted == RowSelection::Kept,
        std::borrow::Cow::Owned(target),
    ))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
