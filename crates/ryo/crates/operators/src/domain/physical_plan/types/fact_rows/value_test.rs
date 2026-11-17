//! Lent rows answer their columns; refused rows answer none, never empty columns.
use super::value::FactRows;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::ColumnsView;

/// Whether the facts lend the rows.
type Input = bool;
type Value = Option<usize>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("lent_rows_answer_their_columns"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(Some(0)),
    },
    Case {
        name: CaseName("refused_rows_answer_no_columns"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(None),
    },
];

fn subject(lent: Input) -> Result<Value, Failure> {
    let rows = if lent {
        FactRows::Lent(ColumnsView {
            kinds: &[],
            columns: &[],
        })
    } else {
        FactRows::Refused
    };
    Ok(rows.lent().map(|view| view.columns.len()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
