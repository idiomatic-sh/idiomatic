//! The collect adapter of the rows port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::RowSinkCollectAdapter;
use super::connection::CollectorConnection;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::row_sink::errors::RowSinkError;
use crate::outbound::row_sink::fixtures::contract::check;
use crate::outbound::row_sink::interface::RowSinkPort as _;
use crate::outbound::row_sink::types::{AnswerCounts, KeptRow, LentCell};
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::row_texts::types::{Cell, CellText};

/// A row's cells, lent.
type Input = &'static [Option<Cell<CellText<'static>>>];
/// Whether the row kept is the row given, each cell owned.
type Value = bool;
type Failure = RowSinkError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_is_kept_with_its_cells_typed"),
    word: Word::Succeeds,
    input: || {
        &[
            Some(Cell::File(CellText("a.rs"))),
            None,
            Some(Cell::Number(Integer(7))),
        ]
    },
    expect: Expect::Value(true),
}];

fn subject(cells: Input) -> Result<Value, Failure> {
    let collected = CollectorConnection::default();
    let adapter = RowSinkCollectAdapter::new(collected.clone());
    adapter.row(&mut cells.iter().copied().map(LentCell::from));
    adapter.end(AnswerCounts::default(), &[])?;
    let kept = collected
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .rows
        .clone();
    Ok(kept
        == [KeptRow(Box::new([
            Some(Cell::File("a.rs".into())),
            None,
            Some(Cell::Number(Integer(7))),
        ]))])
}

#[test]
fn succeeds() {
    check(&RowSinkCollectAdapter::new(CollectorConnection::default()));
    run(CASES, Word::Succeeds, subject);
}
