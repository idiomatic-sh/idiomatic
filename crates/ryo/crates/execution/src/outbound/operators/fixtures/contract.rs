//! Behavioral contract shared by the injected operator adapter and test fake.
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::inbound::use_cases::operators::interface::{
    OperatorAnswer, OperatorCall, PortableOperatorFault,
};
use ryo_relations::domain::relations::types::{Integer, Value as CellValue};
use ryo_values::domain::plan_parts::types::ColumnIndex;

type Input = (Vec<CellValue>, Vec<ColumnIndex>);
type Value = [CellValue; 3];
type Failure = PortableOperatorFault;

/// Observable projection order and first missing-column refusal across the port.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("repeated_columns_keep_order"),
        word: Word::Succeeds,
        input: || {
            (
                vec![CellValue::Number(Integer(7)), CellValue::Number(Integer(9))],
                vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
            )
        },
        expect: Expect::Value([
            CellValue::Number(Integer(9)),
            CellValue::Number(Integer(7)),
            CellValue::Number(Integer(9)),
        ]),
    },
    Case {
        name: CaseName("first_absent_column_is_reported"),
        word: Word::Refuses,
        input: || {
            (
                vec![CellValue::Number(Integer(7))],
                vec![ColumnIndex(3), ColumnIndex(2)],
            )
        },
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: 3 }),
    },
];

/// Run the same observable cases for every implementation of the engine port.
///
/// # Panics
///
/// When the implementation changes ordered projection or its refusal.
pub fn check(adapter: &impl OperatorsPort) {
    let subject = |(row, columns): Input| {
        let mut output = Vec::new();
        match adapter.operators(OperatorCall::Project {
            row: &row,
            columns: &columns,
            target: &mut output,
        })? {
            OperatorAnswer::Written => Ok(output.try_into().expect("three projected cells")),
            _ => panic!("project must write a row"),
        }
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
