//! What every `row_sink` adapter must pass.

use super::super::errors::RowSinkError;
use super::super::interface::RowSinkPort;
use super::super::types::{LentCell, Summary};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{Offset, Span};
use ryo_values::domain::plan_parts::types::Heading;
use ryo_values::domain::row_texts::types::{Cell, CellText, RowTotal};

/// What a row of the contract is given: nothing.
type Input = ();
/// Whether the answer ended without a failure.
type Value = bool;
/// What a row of the contract expects when it is refused.
type Failure = RowSinkError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_answer_of_two_rows_ends_well"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl RowSinkPort) {
    run(CASES, Word::Succeeds, |()| {
        adapter.begin(&[Heading("name".into()), Heading("span".into())]);
        let span = |start: u32, end: u32| {
            Some(Cell::Span(Span {
                start: Offset(start),
                end: Offset(end),
            }))
        };
        adapter.row(
            &mut [Some(Cell::Text(CellText("run"))), span(0, 3)]
                .into_iter()
                .map(LentCell::from),
        );
        adapter.row(
            &mut [Some(Cell::Text(CellText("go"))), span(4, 6)]
                .into_iter()
                .map(LentCell::from),
        );
        adapter
            .end(
                Summary {
                    total: RowTotal(2),
                    shown: RowTotal(2),
                },
                &[],
            )
            .map(|()| true)
    });
}
