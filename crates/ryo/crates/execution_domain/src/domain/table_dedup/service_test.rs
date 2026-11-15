//! The `table_dedup` service's cases, on values alone.

use hashbrown::HashTable;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{CellKind, Value as Cell};

use super::service::first_time;
use crate::domain::table_model::types::Addition;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::errors::RowsError;

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, RowsError>;
/// What a scenario answers: a yes as one, or a count.
type Value = usize;
/// Why a scenario is refused.
type Failure = RowsError;

fn drops_the_repeated_row() -> Value {
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    let mut seen = HashTable::new();
    for number in [1, 1, 2, 1] {
        let _: Addition = first_time(&mut table, &mut seen, &[Cell::Number(Integer(number))]);
    }
    table.count.0 * 10 + table.cells.len()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_repeated_row_is_kept_once"),
    word: Word::Succeeds,
    input: || || Ok(drops_the_repeated_row()),
    expect: Expect::Value(22),
}];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

use ryo_values::domain::rows::types::Table;
