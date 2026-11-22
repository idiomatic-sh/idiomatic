//! The test support of the `fact_tables` concept: a file's node cells, a row of any relation, a set answered from rows of
//! cells, and the reads a case compares: what a language's contract test and the engine's cases build fact sets with.

use super::reading::service::cells_of;
use super::service::{begun, finished, request_of, row, writer};
use super::types::{FactCells, FactRowList, FactSet, FactTable};
use crate::domain::facts::types::CellWord;
use ryo_relations::domain::relations::collections::relations::expression_type::item::ExpressionTypeColumn;
use ryo_relations::domain::relations::service::{columns, kind, node_id};
use ryo_relations::domain::relations::types::{CellKind, Column, FileId, Position, Relation};

/// The file of every case.
pub const FILE: FileId = FileId(3);

/// The cell of node `place` of the file.
#[must_use]
pub fn node(place: u32) -> CellWord {
    CellWord(node_id(FILE, Position(place)).0)
}

/// The cells of a row of `relation`: `place` its node in every node column, `serial` every other cell.
#[must_use]
pub fn row_of(relation: Relation, place: u32, serial: u64) -> Vec<CellWord> {
    columns(relation)
        .into_iter()
        .map(|column| match kind(column) {
            CellKind::Node => node(place),
            CellKind::Symbol
            | CellKind::File
            | CellKind::Span
            | CellKind::Number
            | CellKind::Flag => CellWord(serial),
        })
        .collect()
}

/// A set that answers a request of the relations of `answered`, each with its rows, written in the order given.
#[must_use]
pub fn answered(answered: &[(Relation, Vec<Vec<CellWord>>)]) -> FactSet {
    let relations: Vec<Relation> = answered.iter().map(|(relation, _)| *relation).collect();
    let mut set = FactSet::default();
    begun(&mut set, &request_of(&relations));
    for (relation, rows) in answered {
        let mut writing = writer(&mut set, *relation);
        for cells in rows {
            row(&mut writing, FactCells(cells));
        }
        finished(writing);
    }
    set
}

/// How many rows a lookup found.
#[must_use]
pub fn length(rows: FactRowList<'_>) -> u64 {
    u64::try_from(rows.into_iter().count()).unwrap_or(u64::MAX)
}

/// The cells of `column` over the rows a lookup found, in its order.
#[must_use]
pub fn read(table: &FactTable, column: Column, rows: FactRowList<'_>) -> Vec<CellWord> {
    let cells = cells_of(table, column);
    rows.into_iter()
        .filter_map(|at| {
            cells
                .0
                .get(usize::try_from(at.0).unwrap_or(usize::MAX))
                .copied()
        })
        .collect()
}

/// The column `expression-type` is filled by.
pub const EXPRESSION: Column = Column::ExpressionType(ExpressionTypeColumn::Id);

/// Six rows of `expression-type` over three nodes, the nodes out of order and each with several rows: node, then type.
#[must_use]
pub fn expression_types() -> Vec<Vec<CellWord>> {
    [(5, 10), (2, 11), (5, 12), (9, 13), (2, 14), (5, 15)]
        .into_iter()
        .map(|(place, type_)| {
            vec![
                node(place),
                CellWord(1),
                CellWord(type_),
                CellWord(u64::from(FILE.0)),
                CellWord(0),
            ]
        })
        .collect()
}

/// `rows` with the cell at `place` of each row set to the matching cell of `cells`.
#[must_use]
pub fn with_cells(
    mut rows: Vec<Vec<CellWord>>,
    place: usize,
    cells: &[CellWord],
) -> Vec<Vec<CellWord>> {
    for (each_row, cell) in rows.iter_mut().zip(cells) {
        if let Some(each) = each_row.get_mut(place) {
            *each = *cell;
        }
    }
    rows
}

/// A one-relation answer of `rows`.
#[must_use]
pub fn one(relation: Relation, rows: Vec<Vec<CellWord>>) -> FactSet {
    answered(&[(relation, rows)])
}
