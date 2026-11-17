//! A dormant node's layout empties the table it is given into its columns, whatever the table held: no row, and the
//! layout's kinds.
use super::value::NodeLayout;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::{RowCount, Table};
use std::borrow::Cow;

/// The layout, and how many rows of one number the table held before.
type Input = (&'static [CellKind], usize);
/// The emptied table's kinds and rows.
type Value = (Cow<'static, [CellKind]>, RowCount);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_table_with_rows_is_emptied_into_the_layout"),
        word: Word::Succeeds,
        input: || (&[CellKind::Node, CellKind::Symbol], 2),
        expect: Expect::Value((
            Cow::Borrowed(&[CellKind::Node, CellKind::Symbol]),
            RowCount(0),
        )),
    },
    Case {
        name: CaseName("a_layout_of_no_column_empties_into_no_column"),
        word: Word::Succeeds,
        input: || (&[], 1),
        expect: Expect::Value((Cow::Borrowed(&[]), RowCount(0))),
    },
];

fn subject((kinds, held): Input) -> Result<Value, Failure> {
    let mut rows = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for number in 0..held {
        push_row(
            &mut rows,
            &[Cell::Number(Integer(i64::try_from(number).unwrap_or(0)))],
        );
    }
    NodeLayout(kinds).emptied(&mut rows);
    Ok((Cow::Owned(rows.kinds), rows.count))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
