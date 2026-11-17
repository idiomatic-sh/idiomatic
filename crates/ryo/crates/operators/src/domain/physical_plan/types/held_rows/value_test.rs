//! Held rows answer their table; withheld rows answer the engine's token, never empty rows.
use super::value::HeldRows;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::WithheldInput;
use ryo_values::domain::rows::types::{RowCount, Table};

/// Whether the rows are withheld, and with which token.
type Input = Option<WithheldInput>;
type Value = RowCount;
type Failure = WithheldInput;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("held_rows_answer_their_table"),
        word: Word::Succeeds,
        input: || None,
        expect: Expect::Value(RowCount(1)),
    },
    Case {
        name: CaseName("withheld_rows_answer_the_engine_s_token"),
        word: Word::Refuses,
        input: || Some(WithheldInput(7)),
        expect: Expect::Error(WithheldInput(7)),
    },
];

fn subject(withheld: Input) -> Result<Value, Failure> {
    let table = Table {
        count: RowCount(1),
        ..Table::default()
    };
    let rows = match withheld {
        Some(token) => HeldRows::Withheld(token),
        None => HeldRows::Held(&table),
    };
    rows.table().map(|table| table.count)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
