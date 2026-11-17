//! A table meter counts every node table formed, holds the rows the held tables hold now and keeps the most they held at once;
//! a drop of more rows than are held leaves none.
use super::value::{TableMeter, TablesFormed};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::types::RowCount;

/// What a case forms and drops, in order: a positive count forms a table of that many rows, a negative one drops one.
type Input = &'static [i64];
/// The tables formed, the rows held now and the most rows held at once.
type Value = (TablesFormed, RowCount, RowCount);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_peak_is_the_most_rows_held_at_once"),
        word: Word::Succeeds,
        input: || &[3, 2, -3, 1],
        expect: Expect::Value((TablesFormed(3), RowCount(3), RowCount(5))),
    },
    Case {
        name: CaseName("a_drop_of_more_rows_than_held_leaves_none"),
        word: Word::Succeeds,
        input: || &[2, -5],
        expect: Expect::Value((TablesFormed(1), RowCount(0), RowCount(2))),
    },
];

fn subject(steps: Input) -> Result<Value, Failure> {
    let mut meter = TableMeter::default();
    for step in steps {
        let rows = RowCount(usize::try_from(step.unsigned_abs()).map_err(|_| ())?);
        if *step > 0 {
            meter.formed(rows);
        } else {
            meter.dropped(rows);
        }
    }
    Ok((meter.formed, meter.live_rows, meter.peak_rows))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
